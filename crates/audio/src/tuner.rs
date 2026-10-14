//! The tuner: turns a stream of microphone samples into a steady, readable needle.

use crate::pitch::{PitchDetector, PitchEstimate};
use std::collections::VecDeque;
use theory::{Pitch, Tuning};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunerReading {
    pub frequency: f32,
    /// Nearest note (any note).
    pub note: Pitch,
    /// Cents from the nearest note (−50..50).
    pub cents_from_note: f32,
    /// Closest open string (index, 0 = thickest) and cents from it, when the note is
    /// within a few half steps of one.
    pub string: Option<(usize, f32)>,
    pub clarity: f32,
    pub level_db: f32,
}

impl TunerReading {
    /// Within ±5 cents counts as "in tune" (green).
    pub fn in_tune(cents: f32) -> bool {
        cents.abs() <= 5.0
    }
}

pub struct Tuner {
    detector: PitchDetector,
    buffer: VecDeque<f32>,
    hop: usize,
    since_last: usize,
    sample_rate: u32,
    /// Samples received so far (the tuner's clock).
    samples_seen: u64,
    /// When the last trustworthy estimate arrived.
    last_good_at: u64,
    recent: VecDeque<f32>,
    /// A different pitch that must repeat before it replaces the current note
    /// (stops one-frame octave flickers as a string fades).
    jump: Option<(f32, u8)>,
    /// Loudest level of the current note, for a gate that follows the note as it fades.
    peak_db: f32,
    pub tuning: Tuning,
    /// When set, the needle measures against this string instead of auto-detecting.
    pub target_string: Option<usize>,
    reading: Option<TunerReading>,
    /// Clarity needed to start a new note.
    pub min_clarity: f32,
    /// Clarity needed to keep following a note that is already showing.
    pub sustain_clarity: f32,
    /// How long the last reading stays on screen after the sound becomes unclear.
    pub hold_seconds: f32,
}

impl Tuner {
    pub fn new(sample_rate: u32) -> Tuner {
        // 4096 samples ≈ 85 ms at 48 kHz: several periods of the low E string (82 Hz).
        let size = if sample_rate > 60_000 { 8192 } else { 4096 };
        let mut detector = PitchDetector::new(size, sample_rate as f32);
        detector.min_freq = 60.0;
        detector.max_freq = 1200.0;
        // Quiet tails of a ringing string are still useful; the relative gate below
        // keeps room noise out.
        detector.min_level_db = -80.0;
        Tuner {
            detector,
            buffer: VecDeque::with_capacity(size * 2),
            hop: size / 4,
            since_last: 0,
            sample_rate,
            samples_seen: 0,
            last_good_at: 0,
            recent: VecDeque::with_capacity(8),
            jump: None,
            peak_db: -120.0,
            tuning: Tuning::standard(),
            target_string: None,
            reading: None,
            min_clarity: 0.85,
            sustain_clarity: 0.75,
            hold_seconds: 2.5,
        }
    }

    /// The current reading, kept on screen for `hold_seconds` after the note fades.
    pub fn reading(&self) -> Option<TunerReading> {
        let hold = (self.hold_seconds * self.sample_rate as f32) as u64;
        self.reading
            .filter(|_| self.samples_seen.saturating_sub(self.last_good_at) <= hold)
    }

    /// True when the reading was updated in the last ~0.3 s (the note is still being heard).
    pub fn is_live(&self) -> bool {
        self.reading.is_some()
            && self.samples_seen.saturating_sub(self.last_good_at)
                <= self.sample_rate as u64 * 3 / 10
    }

    /// Feeds new samples. Returns true when the reading was updated.
    pub fn push(&mut self, samples: &[f32]) -> bool {
        let size = self.detector.window_size();
        let mut updated = false;
        for &s in samples {
            if self.buffer.len() == size {
                self.buffer.pop_front();
            }
            self.buffer.push_back(s);
            self.since_last += 1;
            self.samples_seen += 1;
            if self.buffer.len() == size && self.since_last >= self.hop {
                self.since_last = 0;
                let window: Vec<f32> = self.buffer.iter().copied().collect();
                let estimate = self.detector.detect(&window);
                updated |= self.update(estimate);
            }
        }
        updated
    }

    fn cents_between(a: f32, b: f32) -> f32 {
        1200.0 * (a / b).log2()
    }

    fn update(&mut self, estimate: Option<PitchEstimate>) -> bool {
        let Some(e) = estimate else { return false };
        let current = self
            .reading
            .map(|r| r.frequency)
            .filter(|_| self.reading().is_some());
        let same_note = current.is_some_and(|f| Self::cents_between(e.frequency, f).abs() < 60.0);

        let accepted = if same_note {
            // Keep following the ringing note while it's clear enough and not buried in noise.
            e.clarity >= self.sustain_clarity && e.level_db > self.peak_db - 40.0
        } else if e.clarity >= self.min_clarity && e.level_db > self.peak_db - 25.0 {
            match current {
                // A new pitch while a note is showing must repeat (3 frames) before it wins,
                // unless it's clearly louder (a fresh pluck).
                Some(_) if e.level_db <= self.peak_db - 3.0 => {
                    let count = match self.jump {
                        Some((f, n)) if Self::cents_between(e.frequency, f).abs() < 60.0 => n + 1,
                        _ => 1,
                    };
                    self.jump = Some((e.frequency, count));
                    count >= 3
                }
                _ => true,
            }
        } else {
            false
        };
        if !accepted {
            return false;
        }

        if !same_note {
            self.recent.clear();
            self.jump = None;
            self.peak_db = e.level_db;
        }
        self.peak_db = self.peak_db.max(e.level_db);
        if self.recent.len() == 5 {
            self.recent.pop_front();
        }
        self.recent.push_back(e.frequency);
        let mut sorted: Vec<f32> = self.recent.iter().copied().collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let freq = sorted[sorted.len() / 2];

        let (note, cents_from_note) = Pitch::nearest(freq as f64);
        let string = match self.target_string {
            Some(s) => Some((s, self.tuning.open[s].cents_to(freq as f64) as f32)),
            None => self.nearest_string(freq),
        };
        self.reading = Some(TunerReading {
            frequency: freq,
            note,
            cents_from_note: cents_from_note as f32,
            string,
            clarity: e.clarity,
            level_db: e.level_db,
        });
        self.last_good_at = self.samples_seen;
        true
    }

    fn nearest_string(&self, freq: f32) -> Option<(usize, f32)> {
        (0..self.tuning.open.len())
            .map(|s| (s, self.tuning.open[s].cents_to(freq as f64) as f32))
            .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .filter(|(_, c)| c.abs() < 300.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{PluckStyle, StringVoice};

    #[test]
    fn tunes_each_open_string() {
        let sr = 48_000u32;
        let t = Tuning::standard();
        for s in 0..6 {
            let f = t.open[s].frequency() as f32;
            let mut tuner = Tuner::new(sr);
            let mut v = StringVoice::new(sr as f32, 5 + s as u32);
            v.pluck(f, PluckStyle::default());
            let x: Vec<f32> = (0..sr as usize).map(|_| v.tick()).collect();
            tuner.push(&x);
            let r = tuner.reading().expect("reading");
            let (string, cents) = r.string.unwrap();
            assert_eq!(string, s);
            assert!(cents.abs() < 3.0, "string {s}: {cents:.1} cents");
        }
    }

    /// A pluck that fades under a quiet noise floor, like a real string into a laptop mic.
    fn fading_pluck(sr: u32, freq: f32, seconds: f32, then_silence: f32) -> Vec<f32> {
        let mut v = StringVoice::new(sr as f32, 21);
        v.pluck(
            freq,
            PluckStyle {
                velocity: 0.3,
                brightness: 0.6,
                muted: false,
            },
        );
        let mut noise = 12345u32;
        let n = (sr as f32 * (seconds + then_silence)) as usize;
        let cut = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| {
                noise ^= noise << 13;
                noise ^= noise >> 17;
                noise ^= noise << 5;
                let hiss = (noise as f32 / u32::MAX as f32 - 0.5) * 0.0006; // about −70 dB
                let tone = if i < cut { v.tick() } else { 0.0 };
                tone + hiss
            })
            .collect()
    }

    #[test]
    fn follows_a_note_as_it_rings_out() {
        // One pluck should keep the needle up for several seconds, not just a moment.
        let sr = 48_000u32;
        let x = fading_pluck(sr, 110.0, 6.0, 0.0);
        let mut tuner = Tuner::new(sr);
        let mut live_frames = 0;
        let mut checked = 0;
        for chunk in x.chunks(sr as usize / 10) {
            tuner.push(chunk);
            checked += 1;
            if tuner.is_live() {
                live_frames += 1;
                let (string, cents) = tuner.reading().unwrap().string.unwrap();
                assert_eq!(string, 1, "stays on the A string");
                assert!(
                    cents.abs() < 5.0,
                    "{cents} at {:.1}s, clarity {:.2}, level {:.0} dB",
                    checked as f32 / 10.0,
                    tuner.reading().unwrap().clarity,
                    tuner.reading().unwrap().level_db
                );
            }
        }
        // Live (still updating) for at least 2 seconds of the ring-out; the hold adds 2.5 s more.
        eprintln!("live for {live_frames} tenths of a second");
        assert!(
            live_frames >= 20,
            "live for only {live_frames} of {checked} tenths of a second"
        );
    }

    #[test]
    fn holds_then_clears_after_silence() {
        let sr = 48_000u32;
        let mut tuner = Tuner::new(sr);
        let mut v = StringVoice::new(sr as f32, 3);
        v.pluck(196.0, PluckStyle::default());
        let note: Vec<f32> = (0..sr as usize / 2).map(|_| v.tick()).collect();
        tuner.push(&note);
        assert!(tuner.reading().is_some());
        // Suddenly silent (string damped): reading is held briefly...
        tuner.push(&vec![0.0; sr as usize]);
        assert!(tuner.reading().is_some(), "held for a moment");
        // ...and then cleared.
        tuner.push(&vec![0.0; sr as usize * 2]);
        assert!(tuner.reading().is_none(), "cleared after the hold time");
    }

    #[test]
    fn detects_flat_string() {
        let sr = 48_000u32;
        // A string 20 cents flat.
        let f = 110.0f32 * 2f32.powf(-20.0 / 1200.0);
        let mut tuner = Tuner::new(sr);
        let mut v = StringVoice::new(sr as f32, 9);
        v.pluck(f, PluckStyle::default());
        let x: Vec<f32> = (0..sr as usize).map(|_| v.tick()).collect();
        tuner.push(&x);
        let (string, cents) = tuner.reading().unwrap().string.unwrap();
        assert_eq!(string, 1);
        assert!((cents + 20.0).abs() < 3.0, "{cents}");
    }
}
