//! Real-time audio output: runs the synth in the audio callback and plays events at
//! exact sample times. The UI reads the engine's clock to draw playheads in sync.

use crate::synth::{Synth, SynthEvent};
use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// An event scheduled for a sample time on the engine clock (0 or past = play now).
#[derive(Clone, Copy, Debug)]
pub struct Scheduled {
    pub at: u64,
    pub event: SynthEvent,
}

/// Shared clock between the audio thread and the UI.
pub struct Clock {
    origin: Instant,
    sample_rate: u32,
    /// Frames rendered before the most recent callback.
    cb_frames: AtomicU64,
    /// When that callback ran (ns since `origin`).
    cb_nanos: AtomicU64,
    /// How long until that callback's first frame reaches the speakers (ns).
    output_latency_nanos: AtomicU64,
}

impl Clock {
    fn new(sample_rate: u32) -> Self {
        Clock {
            origin: Instant::now(),
            sample_rate,
            cb_frames: AtomicU64::new(0),
            cb_nanos: AtomicU64::new(0),
            output_latency_nanos: AtomicU64::new(0),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn origin(&self) -> Instant {
        self.origin
    }

    /// Nanoseconds since the clock's origin.
    pub fn nanos_now(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }

    /// Best estimate of the sample time being rendered right now (smooth between callbacks).
    pub fn now_frames(&self) -> u64 {
        let frames = self.cb_frames.load(Ordering::Acquire);
        let at = self.cb_nanos.load(Ordering::Acquire);
        let elapsed = self.nanos_now().saturating_sub(at);
        frames + elapsed * self.sample_rate as u64 / 1_000_000_000
    }

    /// When a given frame will actually be heard, in ns since origin.
    pub fn frame_heard_at(&self, frame: u64) -> u64 {
        let frames = self.cb_frames.load(Ordering::Acquire) as i64;
        let at = self.cb_nanos.load(Ordering::Acquire) as i64;
        let lat = self.output_latency_nanos.load(Ordering::Acquire) as i64;
        let delta = (frame as i64 - frames) * 1_000_000_000 / self.sample_rate as i64;
        (at + lat + delta).max(0) as u64
    }

    pub fn output_latency_ms(&self) -> f32 {
        self.output_latency_nanos.load(Ordering::Acquire) as f32 / 1e6
    }

    pub fn seconds_to_frames(&self, seconds: f64) -> u64 {
        (seconds * self.sample_rate as f64).round().max(0.0) as u64
    }
}

pub struct AudioEngine {
    _stream: cpal::Stream,
    tx: rtrb::Producer<Scheduled>,
    samples: rtrb::Producer<(std::sync::Arc<[f32]>, f32)>,
    clock: Arc<Clock>,
    device_name: String,
}

const QUEUE: usize = 4096;

impl AudioEngine {
    /// Opens the default output device and starts the synth.
    pub fn start() -> Result<AudioEngine> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No speakers or headphones were found."))?;
        let device_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "Default output".into());
        let supported = device
            .default_output_config()
            .context("Couldn't read the output device settings.")?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let clock = Arc::new(Clock::new(config.sample_rate));
        let (tx, rx) = rtrb::RingBuffer::<Scheduled>::new(QUEUE);
        let (samples, sample_rx) = rtrb::RingBuffer::new(4);

        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, rx, sample_rx, clock.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, rx, sample_rx, clock.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, rx, sample_rx, clock.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, rx, sample_rx, clock.clone()),
            cpal::SampleFormat::F64 => build::<f64>(&device, &config, rx, sample_rx, clock.clone()),
            other => return Err(anyhow!("Unsupported output format {other}")),
        }?;
        stream.play().context("Couldn't start audio playback.")?;
        Ok(AudioEngine {
            _stream: stream,
            tx,
            samples,
            clock,
            device_name,
        })
    }

    pub fn clock(&self) -> Arc<Clock> {
        self.clock.clone()
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn sample_rate(&self) -> u32 {
        self.clock.sample_rate
    }

    pub fn now(&self) -> u64 {
        self.clock.now_frames()
    }

    /// Schedules an event. Returns false if the queue is full (event dropped).
    pub fn send_at(&mut self, at: u64, event: SynthEvent) -> bool {
        self.tx.push(Scheduled { at, event }).is_ok()
    }

    /// Plays an event as soon as possible.
    pub fn send(&mut self, event: SynthEvent) -> bool {
        self.send_at(0, event)
    }

    /// Plays a recording (mono samples at `sample_rate`) right away.
    pub fn play_sample(&mut self, data: std::sync::Arc<[f32]>, sample_rate: f32) -> bool {
        self.samples.push((data, sample_rate)).is_ok()
    }

    /// A little lead time so events scheduled "now" land exactly where intended.
    pub fn soon(&self) -> u64 {
        self.now() + self.clock.seconds_to_frames(0.03)
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut rx: rtrb::Consumer<Scheduled>,
    mut sample_rx: rtrb::Consumer<(Arc<[f32]>, f32)>,
    clock: Arc<Clock>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut synth = Synth::new(config.sample_rate as f32);
    let mut pending: Vec<Scheduled> = Vec::with_capacity(QUEUE);
    let mut rendered: u64 = 0;

    let stream = device.build_output_stream(
        *config,
        move |data: &mut [T], info: &OutputCallbackInfo| {
            let ts = info.timestamp();
            let latency = ts
                .playback
                .checked_duration_since(ts.callback)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            clock.cb_frames.store(rendered, Ordering::Release);
            clock.cb_nanos.store(clock.nanos_now(), Ordering::Release);
            clock.output_latency_nanos.store(latency, Ordering::Release);

            while let Ok((data, rate)) = sample_rx.pop() {
                synth.play_sample(data, rate);
            }
            // Pull new events into the sorted pending list (no allocation: capacity reserved).
            while let Ok(ev) = rx.pop() {
                if ev.event == SynthEvent::ClearScheduled {
                    pending.clear();
                    continue;
                }
                if pending.len() < pending.capacity() {
                    let pos = pending.partition_point(|p| p.at <= ev.at);
                    pending.insert(pos, ev);
                }
            }

            let mut due = 0;
            for (i, frame) in data.chunks_mut(channels).enumerate() {
                let t = rendered + i as u64;
                while due < pending.len() && pending[due].at <= t {
                    synth.handle(pending[due].event);
                    due += 1;
                }
                let (l, r) = synth.tick_stereo();
                match frame {
                    [only] => *only = T::from_sample((l + r) * 0.5),
                    [left, right, rest @ ..] => {
                        *left = T::from_sample(l);
                        *right = T::from_sample(r);
                        for s in rest {
                            *s = T::from_sample((l + r) * 0.5);
                        }
                    }
                    [] => {}
                }
            }
            pending.drain(..due);
            rendered += (data.len() / channels) as u64;
        },
        |err: cpal::Error| eprintln!("audio output error: {err}"),
        None,
    )?;
    Ok(stream)
}
