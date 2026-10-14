//! Judging lead-guitar techniques from the microphone: bends, vibrato, and swing feel.

use crate::pitch::PitchDetector;
use std::collections::VecDeque;

/// A continuous pitch reading every ~20 ms: (frame, frequency).
pub struct PitchTrack {
    detector: PitchDetector,
    buf: VecDeque<f32>,
    hop: usize,
    since: usize,
    position: u64,
}

impl PitchTrack {
    pub fn new(sample_rate: u32) -> PitchTrack {
        let size = if sample_rate > 60_000 { 4096 } else { 2048 };
        let mut detector = PitchDetector::new(size, sample_rate as f32);
        detector.min_freq = 70.0;
        detector.max_freq = 1400.0;
        detector.min_level_db = -60.0;
        PitchTrack {
            detector,
            buf: VecDeque::with_capacity(size),
            hop: size / 2,
            since: 0,
            position: 0,
        }
    }

    pub fn push(&mut self, start: u64, samples: &[f32]) -> Vec<(u64, f32)> {
        let size = self.detector.window_size();
        let mut out = Vec::new();
        self.position = start;
        for &s in samples {
            if self.buf.len() == size {
                self.buf.pop_front();
            }
            self.buf.push_back(s);
            self.position += 1;
            self.since += 1;
            if self.buf.len() == size && self.since >= self.hop {
                self.since = 0;
                let window: Vec<f32> = self.buf.iter().copied().collect();
                if let Some(e) = self.detector.detect(&window).filter(|e| e.clarity > 0.8) {
                    out.push((self.position, e.frequency));
                }
            }
        }
        out
    }
}

fn cents(f: f32, reference: f32) -> f32 {
    1200.0 * (f / reference).log2()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BendVerdict {
    /// Reached the target within ±25 cents.
    InTune { off_cents: f32 },
    /// Didn't push far enough.
    Flat { off_cents: f32 },
    /// Pushed past the target.
    Sharp { off_cents: f32 },
}

/// Watches one bend: the fretted note must sound first, then rise toward the target.
pub struct BendJudge {
    start_hz: f32,
    target_hz: f32,
    started: bool,
    peak_cents: f32,
    holding: usize,
    last_hz: f32,
}

impl BendJudge {
    pub fn new(start_hz: f32, target_hz: f32) -> BendJudge {
        BendJudge {
            start_hz,
            target_hz,
            started: false,
            peak_cents: 0.0,
            holding: 0,
            last_hz: 0.0,
        }
    }

    /// How high the string is bent right now, in cents above the fretted note.
    pub fn current_cents(&self) -> Option<f32> {
        (self.started && self.last_hz > 0.0).then(|| cents(self.last_hz, self.start_hz))
    }

    /// The bend's goal, in cents above the fretted note (100 = half step, 200 = whole step).
    pub fn goal_cents(&self) -> f32 {
        cents(self.target_hz, self.start_hz)
    }

    /// Feed pitch readings. Returns a verdict once the bend has been held near its peak.
    pub fn push(&mut self, readings: &[(u64, f32)]) -> Option<BendVerdict> {
        for &(_, f) in readings {
            // Fold octave errors back near the expected range.
            let mut f = f;
            while f > self.target_hz * 1.6 {
                f /= 2.0;
            }
            while f < self.start_hz / 1.6 {
                f *= 2.0;
            }
            self.last_hz = f;
            let c = cents(f, self.start_hz);
            if !self.started {
                if c.abs() < 40.0 {
                    self.started = true;
                }
                continue;
            }
            if c > self.peak_cents + 5.0 {
                self.peak_cents = c;
                self.holding = 0;
            } else if c > 50.0 && (c - self.peak_cents).abs() <= 15.0 {
                // Holding near the top of the bend.
                self.holding += 1;
                if self.holding >= 4 {
                    let off = self.peak_cents - self.goal_cents();
                    return Some(if off.abs() <= 25.0 {
                        BendVerdict::InTune { off_cents: off }
                    } else if off < 0.0 {
                        BendVerdict::Flat { off_cents: off }
                    } else {
                        BendVerdict::Sharp { off_cents: off }
                    });
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VibratoReading {
    /// Wobbles per second.
    pub rate_hz: f32,
    /// How wide the wobble is, in cents (low to high).
    pub width_cents: f32,
}

impl VibratoReading {
    /// A musical vibrato: 3–9 wobbles a second, 10–120 cents wide.
    pub fn steady(&self) -> bool {
        (3.0..=9.0).contains(&self.rate_hz) && (10.0..=120.0).contains(&self.width_cents)
    }
}

/// Measures vibrato from at least ~1 second of pitch readings on one held note.
pub fn measure_vibrato(readings: &[(u64, f32)], sample_rate: u32) -> Option<VibratoReading> {
    if readings.len() < 12 {
        return None;
    }
    let base = {
        let mut f: Vec<f32> = readings.iter().map(|r| r.1).collect();
        f.sort_by(|a, b| a.total_cmp(b));
        f[f.len() / 2]
    };
    let c: Vec<f32> = readings.iter().map(|r| cents(r.1, base)).collect();
    if c.iter().any(|x| x.abs() > 300.0) {
        return None; // a different note crept in
    }
    let mean = c.iter().sum::<f32>() / c.len() as f32;
    let mut sorted = c.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let width = sorted[sorted.len() * 9 / 10] - sorted[sorted.len() / 10];
    // Count upward crossings of the mean to find the rate.
    let crossings = c.windows(2).filter(|w| w[0] < mean && w[1] >= mean).count();
    let seconds = (readings.last()?.0 - readings.first()?.0) as f32 / sample_rate as f32;
    if seconds <= 0.0 {
        return None;
    }
    Some(VibratoReading {
        rate_hz: crossings as f32 / seconds,
        width_cents: width,
    })
}

/// Where off-beat notes land between beats: 0.5 = straight eighths, ~0.67 = shuffle/swing.
/// `beats_ns` are the beat times; `onsets_ns` are when the learner played.
pub fn swing_position(beats_ns: &[u64], onsets_ns: &[u64]) -> Option<f32> {
    let mut positions = Vec::new();
    for w in beats_ns.windows(2) {
        let (a, b) = (w[0] as f64, w[1] as f64);
        let len = b - a;
        if let Some(o) = onsets_ns
            .iter()
            .map(|o| *o as f64)
            .find(|o| *o > a + len * 0.3 && *o < a + len * 0.85)
        {
            positions.push(((o - a) / len) as f32);
        }
    }
    if positions.len() < 3 {
        return None;
    }
    positions.sort_by(|a, b| a.total_cmp(b));
    Some(positions[positions.len() / 2])
}

/// Plain-language label for a swing position.
pub fn swing_label(position: f32) -> &'static str {
    if position < 0.57 {
        "straight (even)"
    } else if position < 0.76 {
        "swung (long-short)"
    } else {
        "very late"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{PluckStyle, StringVoice};

    fn track_of(
        voice: &mut StringVoice,
        seconds: f32,
        mut during: impl FnMut(&mut StringVoice, usize),
    ) -> Vec<(u64, f32)> {
        let sr = 48_000usize;
        let mut t = PitchTrack::new(sr as u32);
        let mut out = Vec::new();
        let n = (sr as f32 * seconds) as usize;
        let mut samples = Vec::with_capacity(480);
        for i in 0..n {
            during(voice, i);
            samples.push(voice.tick());
            if samples.len() == 480 {
                out.extend(t.push((i + 1 - 480) as u64, &samples));
                samples.clear();
            }
        }
        out
    }

    #[test]
    fn judges_bends() {
        // G (string 3, fret 7 = D4 in Box 1) bent a whole step to E4.
        let start = 293.66;
        let target = 329.63;
        for (bend_to, expect) in [(target, "in tune"), (311.13, "flat"), (349.23, "sharp")] {
            let mut v = StringVoice::new(48_000.0, 3);
            v.pluck(start, PluckStyle::default());
            let track = track_of(&mut v, 1.6, |v, i| {
                if i == 14_400 {
                    v.glide_to(bend_to, 0.15);
                }
            });
            let mut judge = BendJudge::new(start, target);
            let verdict = judge.push(&track).expect("a verdict");
            let got = match verdict {
                BendVerdict::InTune { .. } => "in tune",
                BendVerdict::Flat { .. } => "flat",
                BendVerdict::Sharp { .. } => "sharp",
            };
            assert_eq!(got, expect, "bend to {bend_to}: {verdict:?}");
        }
    }

    #[test]
    fn measures_vibrato() {
        let mut v = StringVoice::new(48_000.0, 6);
        v.pluck(220.0, PluckStyle::default());
        v.vibrato(40.0, 5.0, 3.0);
        let track = track_of(&mut v, 1.5, |_, _| {});
        let r = measure_vibrato(&track[5..], 48_000).unwrap();
        assert!(r.steady(), "{r:?}");
        assert!((r.rate_hz - 5.0).abs() < 1.5, "{r:?}");

        // No vibrato: not steady (too narrow).
        let mut v = StringVoice::new(48_000.0, 6);
        v.pluck(220.0, PluckStyle::default());
        let track = track_of(&mut v, 1.5, |_, _| {});
        let r = measure_vibrato(&track[5..], 48_000).unwrap();
        assert!(!r.steady(), "{r:?}");
    }

    #[test]
    fn tells_straight_from_swung() {
        let beats: Vec<u64> = (0..8).map(|i| i * 500_000_000).collect();
        let straight: Vec<u64> = beats.iter().flat_map(|b| [*b, b + 250_000_000]).collect();
        let swung: Vec<u64> = beats.iter().flat_map(|b| [*b, b + 333_000_000]).collect();
        assert_eq!(
            swing_label(swing_position(&beats, &straight).unwrap()),
            "straight (even)"
        );
        assert_eq!(
            swing_label(swing_position(&beats, &swung).unwrap()),
            "swung (long-short)"
        );
    }
}
