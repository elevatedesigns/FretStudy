//! App-side wrappers around the audio crate: playing chords and notes (with "ringing"
//! glow for the diagrams), and owning the microphone.

use audio::onset::OnsetDetector;
use audio::play::{self, StrumDirection};
use audio::{AudioEngine, InputDeviceInfo, MicInput, PluckStyle, SynthEvent};
use std::time::Duration;
use theory::{ChordShape, Tuning};

/// How long a string glows on the diagram after it's plucked.
const GLOW_SECONDS: f64 = 0.9;

pub struct Sound {
    engine: Option<AudioEngine>,
    pub error: Option<String>,
    /// When each string was last heard (ns on the shared clock).
    ringing: [Option<u64>; 6],
    /// Scheduled single notes for playheads: (heard_at_ns, string, fret).
    pub sequence: Vec<(u64, usize, u8)>,
    /// Heard times of strum-grid slots being demonstrated (for the grid playhead).
    pub grid: Vec<u64>,
    /// Heard times of metronome beats (for the beat pulse).
    pub metronome: Vec<u64>,
    /// Beats per bar of the running metronome (the accented click).
    pub metronome_bar: usize,
}

impl Sound {
    pub fn start() -> Sound {
        match AudioEngine::start() {
            Ok(engine) => Sound {
                engine: Some(engine),
                error: None,
                ringing: [None; 6],
                sequence: Vec::new(),
                grid: Vec::new(),
                metronome: Vec::new(),
                metronome_bar: 4,
            },
            Err(e) => Sound {
                engine: None,
                error: Some(format!("Sound is off: {e:#}")),
                ringing: [None; 6],
                sequence: Vec::new(),
                grid: Vec::new(),
                metronome: Vec::new(),
                metronome_bar: 4,
            },
        }
    }

    /// No audio device at all (used by visual tests so they don't depend on the machine).
    #[cfg(test)]
    pub fn silent() -> Sound {
        Sound {
            engine: None,
            error: None,
            ringing: [None; 6],
            sequence: Vec::new(),
            grid: Vec::new(),
            metronome: Vec::new(),
            metronome_bar: 4,
        }
    }

    pub fn engine(&self) -> Option<&AudioEngine> {
        self.engine.as_ref()
    }

    pub fn now_ns(&self) -> u64 {
        self.engine
            .as_ref()
            .map(|e| e.clock().nanos_now())
            .unwrap_or(0)
    }

    fn send_all(&mut self, events: Vec<(u64, SynthEvent)>) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        let clock = engine.clock();
        for (at, event) in events {
            engine.send_at(at, event);
            if let SynthEvent::Pluck { string, .. } = event
                && string < 6
            {
                self.ringing[string] = Some(clock.frame_heard_at(at));
            }
        }
    }

    pub fn strum(&mut self, shape: &ChordShape, tuning: &Tuning, direction: StrumDirection) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let at = engine.soon();
        let spread = engine.clock().seconds_to_frames(0.014);
        let style = PluckStyle {
            velocity: 0.85,
            brightness: 0.55,
            muted: false,
        };
        let events = play::strum(shape, tuning, at, direction, spread, style);
        self.send_all(events);
    }

    pub fn arpeggio(&mut self, shape: &ChordShape, tuning: &Tuning) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let at = engine.soon();
        let gap = engine.clock().seconds_to_frames(0.35);
        let events = play::arpeggio(shape, tuning, at, gap, PluckStyle::default());
        self.send_all(events);
    }

    pub fn note(&mut self, tuning: &Tuning, string: usize, fret: u8) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let at = engine.soon();
        self.send_all(vec![(
            at,
            play::note(tuning, string, fret, PluckStyle::default()),
        )]);
    }

    /// Plays notes one after another (`gap` seconds apart) and remembers them for playheads.
    pub fn play_sequence(&mut self, tuning: &Tuning, notes: &[(usize, u8)], gap: f64) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let clock = engine.clock();
        let start = engine.soon();
        let step = clock.seconds_to_frames(gap);
        let mut events = Vec::new();
        self.sequence.clear();
        for (i, &(s, f)) in notes.iter().enumerate() {
            let at = start + step * i as u64;
            events.push((at, play::note(tuning, s, f, PluckStyle::default())));
            self.sequence.push((clock.frame_heard_at(at), s, f));
        }
        self.send_all(events);
    }

    /// The sequence note sounding right now, if any.
    pub fn sequence_position(&self) -> Option<(usize, u8)> {
        let now = self.now_ns();
        let current = self.sequence.iter().rev().find(|(t, _, _)| *t <= now)?;
        let last = self.sequence.last()?;
        (now <= last.0 + 600_000_000).then_some((current.1, current.2))
    }

    pub fn stop(&mut self) {
        self.sequence.clear();
        if let Some(e) = self.engine.as_mut() {
            e.send(SynthEvent::StopAll);
        }
    }

    // ---- Scheduling helpers for lessons (metronomes, play-alongs, demos) ----

    pub fn sample_rate(&self) -> u32 {
        self.engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(48_000)
    }

    /// A frame a little in the future to start something, `lead` seconds from now.
    pub fn frame_after(&self, lead: f64) -> Option<u64> {
        let e = self.engine.as_ref()?;
        Some(e.now() + e.clock().seconds_to_frames(lead))
    }

    pub fn frames_for(&self, seconds: f64) -> u64 {
        (seconds * self.sample_rate() as f64).round() as u64
    }

    /// When a frame will be heard, in ns on the shared clock.
    pub fn heard_ns(&self, frame: u64) -> u64 {
        self.engine
            .as_ref()
            .map(|e| e.clock().frame_heard_at(frame))
            .unwrap_or(0)
    }

    pub fn schedule_strum(
        &mut self,
        shape: &ChordShape,
        tuning: &Tuning,
        frame: u64,
        direction: StrumDirection,
        velocity: f32,
    ) {
        let spread = self.frames_for(if direction == StrumDirection::Up {
            0.010
        } else {
            0.014
        });
        let style = PluckStyle {
            velocity,
            brightness: 0.55,
            muted: false,
        };
        let events = play::strum(shape, tuning, frame, direction, spread, style);
        self.send_all(events);
    }

    /// Plays one strum-grid slot on a chord shape: a strum, a bass note, or a single string.
    pub fn schedule_stroke(
        &mut self,
        shape: &ChordShape,
        tuning: &Tuning,
        frame: u64,
        stroke: content::Stroke,
        velocity: f32,
    ) {
        match stroke {
            content::Stroke::Down => {
                self.schedule_strum(shape, tuning, frame, StrumDirection::Down, velocity)
            }
            content::Stroke::Up => {
                self.schedule_strum(shape, tuning, frame, StrumDirection::Up, velocity * 0.75)
            }
            content::Stroke::Ghost => {}
            content::Stroke::Boogie(_) => {
                let style = PluckStyle {
                    velocity,
                    brightness: 0.4,
                    muted: true,
                };
                let events = boogie_notes(shape, stroke)
                    .into_iter()
                    .map(|(s, f)| (frame, play::note(tuning, s, f, style)))
                    .collect();
                self.send_all(events);
            }
            other => {
                if let Some((s, f)) = stroke_note(shape, other) {
                    let style = PluckStyle {
                        velocity,
                        brightness: 0.45,
                        muted: false,
                    };
                    self.send_all(vec![(frame, play::note(tuning, s, f, style))]);
                }
            }
        }
    }

    /// Plays MIDI pitches (not tied to a chord shape), `gap` seconds apart (0 = strummed together).
    pub fn play_pitches(&mut self, midis: &[i32], gap: f64) {
        let Some(start) = self.frame_after(0.05) else {
            return;
        };
        let step = self.frames_for(if gap <= 0.0 { 0.015 } else { gap });
        let events = midis
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let freq = theory::Pitch(*m).frequency() as f32;
                (
                    start + step * i as u64,
                    SynthEvent::Pluck {
                        string: i % 6,
                        freq,
                        style: PluckStyle::default(),
                    },
                )
            })
            .collect();
        self.send_all(events);
    }

    /// Strums library chords one after another, `gap` seconds apart.
    pub fn chords(&mut self, tuning: &Tuning, shapes: &[ChordShape], gap: f64) {
        let Some(start) = self.frame_after(0.05) else {
            return;
        };
        let step = self.frames_for(gap);
        for (i, shape) in shapes.iter().enumerate() {
            self.schedule_strum(
                shape,
                tuning,
                start + step * i as u64,
                StrumDirection::Down,
                0.85,
            );
        }
    }

    /// Plays notes with rhythm: (string index, fret, beats) at `bpm`, for tab playheads.
    pub fn play_timed(&mut self, tuning: &Tuning, notes: &[(usize, u8, f32)], bpm: f32) {
        let with_tech: Vec<TimedNote> = notes.iter().map(|&(s, f, b)| (s, f, b, None)).collect();
        self.play_tab(tuning, &with_tech, bpm);
    }

    /// Plays tab with techniques: hammer-ons, pull-offs, and slides change pitch without a
    /// new pick; bends glide up after the pick; vibrato wobbles the held note.
    pub fn play_tab(&mut self, tuning: &Tuning, notes: &[TimedNote], bpm: f32) {
        let Some(start) = self.frame_after(0.1) else {
            return;
        };
        let beat = 60.0 / bpm as f64;
        let mut t = 0.0f64;
        let mut events = Vec::new();
        let mut seq = Vec::new();
        let mut prev_string: Option<usize> = None;
        for &(s, f, beats, tech) in notes {
            let frame = start + self.frames_for(t);
            let dur = beat * beats.max(0.25) as f64;
            let freq = tuning.pitch(s, f).frequency() as f32;
            let pick = play::note(tuning, s, f, PluckStyle::default());
            match tech {
                Some(content::Tech::Hammer | content::Tech::Pull) if prev_string == Some(s) => {
                    let accent = if tech == Some(content::Tech::Hammer) {
                        0.6
                    } else {
                        0.3
                    };
                    events.push((
                        frame,
                        SynthEvent::Legato {
                            string: s,
                            freq,
                            seconds: 0.0,
                            accent,
                        },
                    ));
                }
                Some(content::Tech::Slide) if prev_string == Some(s) => {
                    let seconds = (dur * 0.3).min(0.12) as f32;
                    events.push((
                        frame,
                        SynthEvent::Legato {
                            string: s,
                            freq,
                            seconds,
                            accent: 0.0,
                        },
                    ));
                }
                Some(content::Tech::Bend(n)) => {
                    events.push((frame, pick));
                    let target = tuning.pitch(s, f + n).frequency() as f32;
                    let at = frame + self.frames_for((dur * 0.25).min(0.15));
                    let seconds = (dur * 0.35).min(0.25) as f32;
                    events.push((
                        at,
                        SynthEvent::Legato {
                            string: s,
                            freq: target,
                            seconds,
                            accent: 0.0,
                        },
                    ));
                }
                Some(content::Tech::Vibrato) => {
                    events.push((frame, pick));
                    let at = frame + self.frames_for(0.12);
                    events.push((
                        at,
                        SynthEvent::Vibrato {
                            string: s,
                            cents: 30.0,
                            rate: 5.5,
                            seconds: dur as f32,
                        },
                    ));
                }
                _ => events.push((frame, pick)),
            }
            seq.push((self.heard_ns(frame), s, f));
            prev_string = Some(s);
            t += dur;
        }
        self.send_all(events);
        self.sequence = seq;
    }

    /// Plays MIDI pitches at offsets in seconds (for rhythm and progression ear games).
    pub fn play_pitches_at(&mut self, notes: &[(i32, f64)]) {
        let Some(start) = self.frame_after(0.1) else {
            return;
        };
        let events = notes
            .iter()
            .enumerate()
            .map(|(i, (m, at))| {
                let freq = theory::Pitch(*m).frequency() as f32;
                let style = PluckStyle::default();
                (
                    start + self.frames_for(*at),
                    SynthEvent::Pluck {
                        string: i % 6,
                        freq,
                        style,
                    },
                )
            })
            .collect();
        self.send_all(events);
    }

    /// Index of the sequence note sounding now (tab playhead).
    pub fn sequence_index(&self) -> Option<usize> {
        let now = self.now_ns();
        let last = self.sequence.last()?;
        if now > last.0 + 700_000_000 {
            return None;
        }
        self.sequence.iter().rposition(|(t, _, _)| *t <= now)
    }

    /// The strum-grid slot (0..slots) being demonstrated right now.
    pub fn grid_slot(&self, slots: usize) -> Option<usize> {
        let now = self.now_ns();
        let last = *self.grid.last()?;
        let slot_len = if self.grid.len() > 1 {
            self.grid[1] - self.grid[0]
        } else {
            300_000_000
        };
        if now > last + slot_len {
            return None;
        }
        self.grid
            .iter()
            .rposition(|t| *t <= now)
            .map(|i| i % slots.max(1))
    }

    /// Schedules `beats` metronome clicks (accent every 4) and tracks them for the beat pulse.
    pub fn start_metronome(&mut self, bpm: f32, beats: usize) -> Vec<u64> {
        self.start_metronome_in(bpm, beats, 4)
    }

    /// Like `start_metronome`, with the accent every `per_bar` beats.
    pub fn start_metronome_in(&mut self, bpm: f32, beats: usize, per_bar: usize) -> Vec<u64> {
        let per_bar = per_bar.max(1);
        self.metronome_bar = per_bar;
        let Some(start) = self.frame_after(0.2) else {
            return Vec::new();
        };
        let step = self.frames_for(60.0 / bpm as f64);
        let mut heard = Vec::new();
        for i in 0..beats {
            if let Some(h) = self.click(start + step * i as u64, i % per_bar == 0) {
                heard.push(h);
            }
        }
        self.metronome = heard.clone();
        heard
    }

    /// Current metronome beat (0..4) and a 0..1 "flash" right after the click.
    pub fn metronome_beat(&self) -> Option<(usize, f32)> {
        let now = self.now_ns();
        let last = *self.metronome.last()?;
        if now > last + 600_000_000 {
            return None;
        }
        let i = self.metronome.iter().rposition(|t| *t <= now)?;
        let age = (now - self.metronome[i]) as f32 / 1e9;
        Some((
            i % self.metronome_bar.max(1),
            (1.0 - age / 0.15).clamp(0.0, 1.0),
        ))
    }

    /// Stops sound and forgets anything scheduled (e.g. leaving an exercise early).
    pub fn cancel_scheduled(&mut self) {
        self.sequence.clear();
        self.grid.clear();
        self.metronome.clear();
        if let Some(e) = self.engine.as_mut() {
            e.send(SynthEvent::ClearScheduled);
            e.send(SynthEvent::StopAll);
        }
    }

    pub fn click(&mut self, at_frame: u64, accent: bool) -> Option<u64> {
        let engine = self.engine.as_mut()?;
        engine.send_at(at_frame, SynthEvent::Click { accent });
        Some(engine.clock().frame_heard_at(at_frame))
    }

    /// Schedules a drum hit (song playback).
    pub fn drum(&mut self, at_frame: u64, drum: audio::Drum) {
        if let Some(engine) = self.engine.as_mut() {
            engine.send_at(at_frame, SynthEvent::Drum { drum });
        }
    }

    /// Plays a recording (mono samples at `sample_rate`) now.
    pub fn play_recording(&mut self, samples: std::sync::Arc<[f32]>, sample_rate: u32) {
        if let Some(engine) = self.engine.as_mut() {
            engine.play_sample(samples, sample_rate as f32);
        }
    }

    /// 0..1 glow per string, fading after each pluck.
    pub fn glow(&self) -> [f32; 6] {
        let now = self.now_ns();
        self.ringing.map(|t| match t {
            Some(t) if now >= t => {
                let age = (now - t) as f64 / 1e9;
                if age < GLOW_SECONDS {
                    (1.0 - age / GLOW_SECONDS) as f32
                } else {
                    0.0
                }
            }
            _ => 0.0,
        })
    }

    pub fn animating(&self) -> bool {
        let now = self.now_ns();
        let pending = |last: Option<u64>| last.is_some_and(|t| t + 700_000_000 >= now);
        self.glow().iter().any(|g| *g > 0.0)
            || self.ringing.iter().flatten().any(|t| *t > now)
            || pending(self.sequence.last().map(|s| s.0))
            || pending(self.grid.last().copied())
            || pending(self.metronome.last().copied())
    }
}

/// Which string and fret a bass or single-string stroke plays on a chord shape.
/// Bass 0 = the lowest played string (usually the root). Bass 1 = the alternate bass:
/// two strings up for 6th-string roots (G → the D string), one string up otherwise.
pub fn stroke_note(shape: &ChordShape, stroke: content::Stroke) -> Option<(usize, u8)> {
    let played: Vec<usize> = (0..6).filter(|s| shape.frets[*s].is_some()).collect();
    let low = *played.first()?;
    let string = match stroke {
        content::Stroke::Bass(0) => low,
        content::Stroke::Bass(_) => {
            let alt = if low == 0 { 2 } else { low + 1 };
            if played.contains(&alt) { alt } else { low }
        }
        content::Stroke::Pluck(i) => {
            let from_top = 4usize.saturating_sub(i as usize);
            played[played
                .len()
                .saturating_sub(1 + from_top)
                .max(usize::from(played.len() > 1))]
        }
        _ => return None,
    };
    Some((string, shape.frets[string]?))
}

/// The two notes of a boogie-shuffle dyad on a chord's root: root + 5th (0) or root + 6th (1).
/// Works for roots on strings 6, 5, and 4 (the next string up is a 4th higher).
pub fn boogie_notes(shape: &ChordShape, stroke: content::Stroke) -> Vec<(usize, u8)> {
    let content::Stroke::Boogie(which) = stroke else {
        return Vec::new();
    };
    let Some(low) = (0..6).find(|s| shape.frets[*s].is_some()) else {
        return Vec::new();
    };
    let root = shape.frets[low].unwrap_or(0);
    if low >= 3 {
        return vec![(low, root)];
    }
    vec![
        (low, root),
        (low + 1, root + if which == 0 { 2 } else { 4 }),
    ]
}

/// (string index, fret, beats, technique) for tab playback.
pub type TimedNote = (usize, u8, f32, Option<content::Tech>);

/// The microphone, shared by the tuner and the sound check.
pub struct Mic {
    pub input: Option<MicInput>,
    pub error: Option<String>,
    pub devices: Vec<InputDeviceInfo>,
    pub selected: Option<String>,
    /// Samples read this frame and the absolute frame index of the first one.
    pub frame_start: u64,
    pub frame_samples: Vec<f32>,
    /// Smoothed input level for the meter (dBFS).
    pub level_db: f32,
    /// Samples captured since `start_recording` (None = not recording).
    pub recording: Option<Vec<f32>>,
}

impl Mic {
    pub fn new() -> Mic {
        Mic {
            input: None,
            error: None,
            devices: Vec::new(),
            selected: None,
            frame_start: 0,
            frame_samples: Vec::new(),
            level_db: -90.0,
            recording: None,
        }
    }

    pub fn refresh_devices(&mut self) {
        self.devices = audio::list_input_devices();
    }

    pub fn is_on(&self) -> bool {
        self.input.is_some()
    }

    pub fn start(&mut self, sound: &Sound) {
        let origin = sound
            .engine()
            .map(|e| e.clock().origin())
            .unwrap_or_else(std::time::Instant::now);
        match MicInput::start(self.selected.as_deref(), origin) {
            Ok(m) => {
                self.input = Some(m);
                self.error = None;
            }
            Err(e) => {
                self.input = None;
                self.error = Some(format!("{e:#}"));
            }
        }
    }

    pub fn stop(&mut self) {
        self.input = None;
        self.level_db = -90.0;
    }

    pub fn sample_rate(&self) -> u32 {
        self.input
            .as_ref()
            .map(|m| m.sample_rate())
            .unwrap_or(48_000)
    }

    /// Reads everything captured since the last frame.
    pub fn poll(&mut self) {
        self.frame_samples.clear();
        if let Some(m) = self.input.as_mut() {
            self.frame_start = m.read(&mut self.frame_samples);
            if let Some(take) = self.recording.as_mut() {
                take.extend_from_slice(&self.frame_samples);
            }
            if !self.frame_samples.is_empty() {
                let ms = self.frame_samples.iter().map(|x| x * x).sum::<f32>()
                    / self.frame_samples.len() as f32;
                let db = 10.0 * ms.max(1e-12).log10();
                // Fast attack, slow release, like a real level meter.
                self.level_db = if db > self.level_db {
                    db
                } else {
                    self.level_db + (db - self.level_db) * 0.15
                };
            }
        }
    }
}

/// Measures the delay between a click being played and the microphone hearing it,
/// so rhythm checks can line up what was heard with what was played.
pub struct Calibration {
    pub heard_ns: Vec<u64>,
    pub onsets_ns: Vec<u64>,
    detector: OnsetDetector,
    end_ns: u64,
    pub running: bool,
    pub result: Option<Result<f32, String>>,
    pub strum_along: bool,
}

impl Calibration {
    pub const CLICKS: usize = 8;

    pub fn new() -> Calibration {
        Calibration {
            heard_ns: Vec::new(),
            onsets_ns: Vec::new(),
            detector: OnsetDetector::new(48_000),
            end_ns: 0,
            running: false,
            result: None,
            strum_along: false,
        }
    }

    pub fn start(&mut self, sound: &mut Sound, mic: &Mic) {
        let Some(engine) = sound.engine() else {
            self.result = Some(Err("Sound is off, so the clicks can't play.".into()));
            return;
        };
        let clock = engine.clock();
        let first = engine.soon() + clock.seconds_to_frames(0.8);
        let step = clock.seconds_to_frames(0.6);
        self.heard_ns.clear();
        self.onsets_ns.clear();
        self.detector = OnsetDetector::new(mic.sample_rate());
        if self.strum_along {
            // A guitar strum is much louder than room noise; be a little less picky.
            self.detector.ratio = 4.0;
        }
        for i in 0..Self::CLICKS {
            if let Some(h) = sound.click(first + step * i as u64, i % 4 == 0) {
                self.heard_ns.push(h);
            }
        }
        self.end_ns = self.heard_ns.last().copied().unwrap_or(0)
            + Duration::from_millis(700).as_nanos() as u64;
        self.running = true;
        self.result = None;
    }

    /// Call every frame with the samples the mic captured this frame.
    pub fn update(&mut self, sound: &Sound, mic: &Mic) {
        if !self.running {
            return;
        }
        if let Some(input) = mic.input.as_ref() {
            for frame in self.detector.push(mic.frame_start, &mic.frame_samples) {
                self.onsets_ns.push(input.frame_captured_at(frame));
            }
        }
        if sound.now_ns() > self.end_ns {
            self.running = false;
            let window = Duration::from_millis(300).as_nanos() as u64;
            self.result = Some(
                match audio::onset::median_offset(&self.heard_ns, &self.onsets_ns, window, 5) {
                    Some(offset) => Ok(offset as f32 / 1e6),
                    None => Err(if self.strum_along {
                        "I couldn't hear enough strums. Strum firmly right on each click and try again.".into()
                    } else {
                        "I couldn't hear the clicks. Turn your speakers up (not headphones) and try again, or use strum-along mode.".into()
                    }),
                },
            );
        }
    }

    pub fn progress(&self, now_ns: u64) -> f32 {
        let (Some(first), Some(_)) = (self.heard_ns.first(), self.heard_ns.last()) else {
            return 0.0;
        };
        let span = self.end_ns.saturating_sub(*first).max(1) as f32;
        (now_ns.saturating_sub(*first) as f32 / span).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod stroke_tests {
    use super::stroke_note;
    use content::Stroke;
    use theory::ChordShape;

    #[test]
    fn bass_and_pluck_strings() {
        let g = ChordShape::parse("320003", None).unwrap();
        assert_eq!(stroke_note(&g, Stroke::Bass(0)), Some((0, 3))); // G on string 6
        assert_eq!(stroke_note(&g, Stroke::Bass(1)), Some((2, 0))); // open D string
        let c = ChordShape::parse("x32010", None).unwrap();
        assert_eq!(stroke_note(&c, Stroke::Bass(0)), Some((1, 3))); // C on string 5
        assert_eq!(stroke_note(&c, Stroke::Bass(1)), Some((2, 2))); // E on string 4
        let am = ChordShape::parse("x02210", None).unwrap();
        assert_eq!(stroke_note(&am, Stroke::Pluck(4)), Some((5, 0))); // thinnest played
        assert_eq!(stroke_note(&am, Stroke::Pluck(2)), Some((3, 2))); // G string
        let d = ChordShape::parse("xx0232", None).unwrap();
        assert_eq!(stroke_note(&d, Stroke::Pluck(4)), Some((5, 2)));
        assert_eq!(stroke_note(&d, Stroke::Pluck(2)), Some((3, 2)));
    }
}
