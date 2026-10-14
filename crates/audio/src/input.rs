//! Microphone capture. Samples are mixed to mono and handed to the UI thread through a
//! lock-free ring buffer, along with timing so any sample can be placed on the shared timeline.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Lists microphones and audio inputs.
pub fn list_input_devices() -> Vec<InputDeviceInfo> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            let id = d.id().ok()?.to_string();
            let name = d
                .description()
                .map(|x| x.name().to_string())
                .unwrap_or_else(|_| id.clone());
            Some(InputDeviceInfo {
                is_default: Some(&id) == default_id.as_ref(),
                id,
                name,
            })
        })
        .collect()
}

/// Timing shared with the audio input thread.
struct InputTiming {
    /// Frames captured before the most recent callback.
    cb_frames: AtomicU64,
    /// When that callback's first frame was captured (ns since the shared origin).
    capture_nanos: AtomicU64,
}

pub struct MicInput {
    _stream: cpal::Stream,
    rx: rtrb::Consumer<f32>,
    sample_rate: u32,
    device_name: String,
    timing: Arc<InputTiming>,
    origin: Instant,
    /// Absolute frame index of the next sample `read()` will return.
    frames_read: u64,
    /// Samples dropped because the UI didn't read fast enough.
    overflow: Arc<AtomicU64>,
}

impl MicInput {
    /// Opens an input device (`None` = system default). `origin` must be the same instant
    /// used by the output clock so input and output times can be compared.
    pub fn start(device_id: Option<&str>, origin: Instant) -> Result<MicInput> {
        let host = cpal::default_host();
        let device = match device_id {
            Some(id) => {
                let parsed = id.parse().map_err(|_| anyhow!("Unknown input device id"))?;
                host.device_by_id(&parsed)
                    .ok_or_else(|| anyhow!("That microphone isn't connected."))?
            }
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow!("No microphone was found."))?,
        };
        let device_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "Microphone".into());
        let supported = device
            .default_input_config()
            .context("Couldn't read the microphone settings.")?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let sample_rate = config.sample_rate;
        // Two seconds of headroom between the audio thread and the UI.
        let (tx, rx) = rtrb::RingBuffer::<f32>::new(sample_rate as usize * 2);
        let timing = Arc::new(InputTiming {
            cb_frames: AtomicU64::new(0),
            capture_nanos: AtomicU64::new(0),
        });
        let overflow = Arc::new(AtomicU64::new(0));

        let ctx = Ctx {
            tx,
            timing: timing.clone(),
            overflow: overflow.clone(),
            origin,
            channels: config.channels as usize,
            captured: 0,
        };
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, ctx),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, ctx),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, ctx),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, ctx),
            cpal::SampleFormat::F64 => build::<f64>(&device, &config, ctx),
            other => return Err(anyhow!("Unsupported microphone format {other}")),
        }
        .map_err(|e| anyhow!(permission_hint(&e.to_string())))?;
        stream
            .play()
            .map_err(|e| anyhow!(permission_hint(&e.to_string())))?;
        Ok(MicInput {
            _stream: stream,
            rx,
            sample_rate,
            device_name,
            timing,
            origin,
            frames_read: 0,
            overflow,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Appends all newly captured samples to `out`. Returns the absolute frame index of
    /// the first appended sample.
    pub fn read(&mut self, out: &mut Vec<f32>) -> u64 {
        let start = self.frames_read;
        let n = self.rx.slots();
        if n > 0
            && let Ok(chunk) = self.rx.read_chunk(n)
        {
            let (a, b) = chunk.as_slices();
            out.extend_from_slice(a);
            out.extend_from_slice(b);
            chunk.commit_all();
            self.frames_read += n as u64;
        }
        start
    }

    /// When an input frame was captured, in ns since the shared origin.
    pub fn frame_captured_at(&self, frame: u64) -> u64 {
        let cb_frames = self.timing.cb_frames.load(Ordering::Acquire) as i64;
        let cap = self.timing.capture_nanos.load(Ordering::Acquire) as i64;
        let delta = (frame as i64 - cb_frames) * 1_000_000_000 / self.sample_rate as i64;
        (cap + delta).max(0) as u64
    }

    pub fn dropped_samples(&self) -> u64 {
        self.overflow.load(Ordering::Relaxed)
    }

    pub fn origin(&self) -> Instant {
        self.origin
    }
}

struct Ctx {
    tx: rtrb::Producer<f32>,
    timing: Arc<InputTiming>,
    overflow: Arc<AtomicU64>,
    origin: Instant,
    channels: usize,
    captured: u64,
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut ctx: Ctx,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: cpal::FromSample<T>,
{
    device.build_input_stream(
        *config,
        move |data: &[T], info: &InputCallbackInfo| {
            let ts = info.timestamp();
            let age = ts
                .callback
                .checked_duration_since(ts.capture)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let now = ctx.origin.elapsed().as_nanos() as u64;
            ctx.timing.cb_frames.store(ctx.captured, Ordering::Release);
            ctx.timing
                .capture_nanos
                .store(now.saturating_sub(age), Ordering::Release);
            let ch = ctx.channels.max(1);
            for frame in data.chunks(ch) {
                let mut sum = 0.0f32;
                for s in frame {
                    sum += <f32 as cpal::FromSample<T>>::from_sample_(*s);
                }
                if ctx.tx.push(sum / ch as f32).is_err() {
                    ctx.overflow.fetch_add(1, Ordering::Relaxed);
                }
            }
            ctx.captured += (data.len() / ch) as u64;
        },
        |err: cpal::Error| eprintln!("microphone error: {err}"),
        None,
    )
}

fn permission_hint(raw: &str) -> String {
    let os_hint = if cfg!(target_os = "macos") {
        "On a Mac: open System Settings → Privacy & Security → Microphone and turn on FretStudy (or your Terminal app while developing)."
    } else if cfg!(target_os = "windows") {
        "On Windows: open Settings → Privacy & security → Microphone and turn on \"Let desktop apps access your microphone\"."
    } else {
        "Check that your system allows apps to use the microphone."
    };
    format!("The microphone couldn't start ({raw}). {os_hint}")
}
