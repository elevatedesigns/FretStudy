//! Strummed-chord checking. FretStudy always knows which shape the learner is holding, so
//! it *verifies* that shape instead of guessing the chord from scratch:
//!
//! 1. Each played string's note should be in the sound (its fundamental, or — because
//!    laptop mics barely hear low notes — its 2nd and 3rd harmonics).
//! 2. Any strong sound that none of the expected notes can explain (as a note or one of its
//!    overtones) is "extra": a wrong finger or the wrong chord.

use crate::onset::OnsetDetector;
use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

/// A spectral peak: (frequency Hz, magnitude relative to the strongest peak).
pub type Peak = (f32, f32);

pub struct SpectrumAnalyzer {
    size: usize,
    sample_rate: f32,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> SpectrumAnalyzer {
        // ~170 ms at 48 kHz: fine enough to tell neighboring low notes apart.
        let size = if sample_rate > 60_000 { 16_384 } else { 8192 };
        let fft = FftPlanner::new().plan_fft_forward(size);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let window = (0..size)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / size as f32).cos())
            .collect();
        SpectrumAnalyzer {
            size,
            sample_rate: sample_rate as f32,
            fft,
            window,
            buf: vec![Complex32::default(); size],
            scratch,
        }
    }

    pub fn window_size(&self) -> usize {
        self.size
    }

    /// Spectral peaks between 60 Hz and 3 kHz, with sub-bin frequency accuracy.
    pub fn peaks(&mut self, samples: &[f32]) -> Vec<Peak> {
        for i in 0..self.size {
            let x = samples.get(i).copied().unwrap_or(0.0);
            self.buf[i] = Complex32::new(x * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);
        let bin_hz = self.sample_rate / self.size as f32;
        let mags: Vec<f32> = self.buf[..self.size / 2].iter().map(|c| c.norm()).collect();
        let lo = ((60.0 / bin_hz) as usize).max(1);
        let hi = ((3000.0 / bin_hz) as usize).min(mags.len() - 2);
        let top = mags[lo..hi]
            .iter()
            .cloned()
            .fold(0.0f32, f32::max)
            .max(1e-9);
        let mut peaks = Vec::new();
        for k in lo..hi {
            let m = mags[k];
            if m < top * 0.01 || m < mags[k - 1] || m < mags[k + 1] {
                continue;
            }
            let (a, b, c) = (
                mags[k - 1].max(1e-12).ln(),
                m.ln(),
                mags[k + 1].max(1e-12).ln(),
            );
            let denom = a - 2.0 * b + c;
            let shift = if denom.abs() > 1e-9 {
                0.5 * (a - c) / denom
            } else {
                0.0
            };
            peaks.push(((k as f32 + shift) * bin_hz, m / top));
        }
        peaks
    }
}

/// How far (as a ratio) a peak may be from a target frequency and still match it. 2.5% is
/// a little less than half a semitone.
const MATCH: f32 = 0.025;

fn strength_near(peaks: &[Peak], target: f32) -> f32 {
    peaks
        .iter()
        .filter(|p| (p.0 / target - 1.0).abs() < MATCH)
        .map(|p| p.1)
        .fold(0.0, f32::max)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeVerdict {
    /// Indexes (into the expected list) of notes that weren't heard.
    pub missing: Vec<usize>,
    /// Frequencies of strong sounds the chord doesn't explain.
    pub extra: Vec<f32>,
}

impl ShapeVerdict {
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty()
    }
}

/// Checks a strum against the expected note frequencies (one per played string).
pub fn verify_shape(peaks: &[Peak], expected: &[f32]) -> ShapeVerdict {
    let present = 0.03;
    let missing = expected
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            let fundamental = strength_near(peaks, **f);
            // Small mics barely pick up the lowest notes, so for low strings the 2nd and 3rd
            // harmonics can stand in for the fundamental. (Not for higher strings: there,
            // other notes' overtones could fake it.)
            let overtones = if **f < 150.0 {
                strength_near(peaks, *f * 2.0).min(strength_near(peaks, *f * 3.0))
            } else {
                0.0
            };
            fundamental < present && overtones < present * 2.0
        })
        .map(|(i, _)| i)
        .collect();
    let explained = |f: f32| {
        expected.iter().any(|e| {
            let ratio = f / e;
            let h = ratio.round();
            (1.0..=16.0).contains(&h) && (ratio / h - 1.0).abs() < MATCH
        })
    };
    // Wrong notes show up as strong low peaks; very high peaks are mostly overtones.
    let extra = peaks
        .iter()
        .filter(|p| p.1 >= 0.25 && p.0 < 800.0 && !explained(p.0))
        .map(|p| p.0)
        .collect();
    ShapeVerdict { missing, extra }
}

/// Listens for strums and returns the spectral peaks of each one.
pub struct StrumListener {
    onsets: OnsetDetector,
    analyzer: SpectrumAnalyzer,
    collecting: Option<(u64, Vec<f32>)>,
    delay: u64,
}

impl StrumListener {
    pub fn new(sample_rate: u32) -> StrumListener {
        let mut onsets = OnsetDetector::new(sample_rate);
        onsets.ratio = 4.0;
        StrumListener {
            onsets,
            analyzer: SpectrumAnalyzer::new(sample_rate),
            collecting: None,
            // Skip the noisy pick attack and let every string of the strum sound (~80 ms).
            delay: sample_rate as u64 * 80 / 1000,
        }
    }

    /// Feeds samples starting at absolute frame `start`. Returns (onset frame, peaks) for
    /// each strum whose analysis window is complete.
    pub fn push(&mut self, start: u64, samples: &[f32]) -> Vec<(u64, Vec<Peak>)> {
        let mut out = Vec::new();
        let onsets = self.onsets.push(start, samples);
        let need = self.analyzer.window_size();
        // An onset can be dated a little before this chunk (it's the first loud sample of
        // the energy frame), so start collecting from it when it's reported. Ringing and room
        // echoes can look like new onsets; a strum being analyzed finishes first.
        if self.collecting.is_none()
            && let Some(&o) = onsets.first()
        {
            self.collecting = Some((o, Vec::with_capacity(need)));
        }
        for (i, &s) in samples.iter().enumerate() {
            let frame = start + i as u64;
            if let Some((onset, buf)) = self.collecting.as_mut()
                && frame >= *onset + self.delay
            {
                buf.push(s);
                if buf.len() >= need {
                    let (onset, buf) = self.collecting.take().unwrap();
                    out.push((onset, self.analyzer.peaks(&buf)));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{StrumDirection, strum};
    use crate::synth::{PluckStyle, Synth};
    use theory::{ChordShape, Tuning};

    /// Synth strum of `played` frets, rendered to samples.
    fn render(frets: &str) -> Vec<f32> {
        let shape = ChordShape::parse(frets, None).unwrap();
        let t = Tuning::standard();
        let mut events = strum(
            &shape,
            &t,
            4800,
            StrumDirection::Down,
            600,
            PluckStyle::default(),
        );
        events.sort_by_key(|e| e.0);
        let mut synth = Synth::new(48_000.0);
        let mut out = Vec::new();
        let mut next = 0;
        for i in 0..72_000 {
            while next < events.len() && events[next].0 as usize <= i {
                synth.handle(events[next].1);
                next += 1;
            }
            out.push(synth.tick());
        }
        out
    }

    fn expected(frets: &str) -> Vec<f32> {
        let shape = ChordShape::parse(frets, None).unwrap();
        shape
            .pitches(&Tuning::standard())
            .iter()
            .map(|(_, p)| p.frequency() as f32)
            .collect()
    }

    fn check(played: &str, expect: &str) -> ShapeVerdict {
        let mut l = StrumListener::new(48_000);
        let samples = render(played);
        let mut found = Vec::new();
        for (i, chunk) in samples.chunks(480).enumerate() {
            found.extend(l.push((i * 480) as u64, chunk));
        }
        assert!(!found.is_empty(), "no strum heard");
        verify_shape(&found[0].1, &expected(expect))
    }

    #[test]
    fn correct_chords_pass() {
        for frets in [
            "022000", "022100", "x32010", "320003", "xx0232", "x02210", "xx0212", "133211",
            "x02220", "xx0231", "x24432",
        ] {
            let v = check(frets, frets);
            assert!(v.ok(), "{frets}: {v:?}");
        }
    }

    #[test]
    fn wrong_chords_fail() {
        assert!(!check("x02210", "x32010").ok(), "Am played, C expected");
        assert!(!check("022000", "022100").ok(), "Em played, E expected");
        assert!(!check("xx0231", "xx0232").ok(), "Dm played, D expected");
    }

    #[test]
    fn a_dead_string_is_named() {
        // G7 expected, but string 1 (F, fret 1) didn't sound.
        let v = check("32000x", "320001");
        assert_eq!(
            v.missing,
            vec![5],
            "the 6th expected note (string 1) is missing: {v:?}"
        );
    }
}
