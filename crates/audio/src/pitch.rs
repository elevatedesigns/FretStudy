//! Single-note pitch detection with the McLeod Pitch Method (MPM): a normalized
//! square-difference function computed with FFTs, then "first good peak" picking.

use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchEstimate {
    pub frequency: f32,
    /// 0..1 — how periodic (clean) the signal is. Above ~0.9 is a clear note.
    pub clarity: f32,
    /// Signal level in dBFS (RMS).
    pub level_db: f32,
}

pub struct PitchDetector {
    size: usize,
    sample_rate: f32,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    spectrum: Vec<Complex32>,
    scratch: Vec<Complex32>,
    nsdf: Vec<f32>,
    pub min_freq: f32,
    pub max_freq: f32,
    /// Peaks within this fraction of the highest peak count as "good" (MPM's k).
    pub threshold: f32,
    /// Ignore signals quieter than this (dBFS).
    pub min_level_db: f32,
}

impl PitchDetector {
    /// `size` is the analysis window in samples (2048–4096 at 44.1/48 kHz works well for guitar).
    pub fn new(size: usize, sample_rate: f32) -> Self {
        let mut planner = FftPlanner::new();
        let n = (size * 2).next_power_of_two();
        let forward = planner.plan_fft_forward(n);
        let inverse = planner.plan_fft_inverse(n);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        PitchDetector {
            size,
            sample_rate,
            forward,
            inverse,
            spectrum: vec![Complex32::default(); n],
            scratch: vec![Complex32::default(); scratch_len],
            nsdf: vec![0.0; size],
            min_freq: 60.0,
            max_freq: 1400.0,
            threshold: 0.9,
            min_level_db: -55.0,
        }
    }

    pub fn window_size(&self) -> usize {
        self.size
    }

    /// Analyzes the most recent `window_size()` samples of `signal`.
    pub fn detect(&mut self, signal: &[f32]) -> Option<PitchEstimate> {
        if signal.len() < self.size {
            return None;
        }
        let x = &signal[signal.len() - self.size..];
        let energy: f32 = x.iter().map(|v| v * v).sum();
        let rms = (energy / self.size as f32).sqrt();
        let level_db = 20.0 * rms.max(1e-9).log10();
        if level_db < self.min_level_db {
            return None;
        }

        // Autocorrelation r(τ) via FFT (zero-padded to avoid wrap-around).
        let n = self.spectrum.len();
        for (i, c) in self.spectrum.iter_mut().enumerate() {
            *c = Complex32::new(if i < self.size { x[i] } else { 0.0 }, 0.0);
        }
        self.forward
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);
        for c in self.spectrum.iter_mut() {
            *c = Complex32::new(c.norm_sqr(), 0.0);
        }
        self.inverse
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);
        let scale = 1.0 / n as f32;

        // m(τ) = Σ (x_j² + x_{j+τ}²), updated incrementally. nsdf(τ) = 2 r(τ) / m(τ).
        let mut m = 2.0 * energy;
        for tau in 0..self.size {
            if tau > 0 {
                m -= x[tau - 1] * x[tau - 1] + x[self.size - tau] * x[self.size - tau];
            }
            let r = self.spectrum[tau].re * scale;
            self.nsdf[tau] = if m > 1e-12 { 2.0 * r / m } else { 0.0 };
        }

        let min_tau = (self.sample_rate / self.max_freq).floor() as usize;
        let max_tau = ((self.sample_rate / self.min_freq).ceil() as usize).min(self.size - 2);

        // Key maxima: the highest point between each positive-going and negative-going zero crossing.
        let mut peaks: Vec<(usize, f32)> = Vec::new();
        let mut tau = 1;
        while tau < max_tau && self.nsdf[tau] > 0.0 {
            tau += 1; // skip the initial lobe around τ = 0
        }
        while tau < max_tau {
            while tau < max_tau && self.nsdf[tau] <= 0.0 {
                tau += 1;
            }
            let mut best = (tau, f32::MIN);
            while tau < max_tau && self.nsdf[tau] > 0.0 {
                if self.nsdf[tau] > best.1 {
                    best = (tau, self.nsdf[tau]);
                }
                tau += 1;
            }
            if best.1 > f32::MIN && best.0 >= min_tau {
                peaks.push(best);
            }
        }
        let highest = peaks.iter().map(|p| p.1).fold(0.0f32, f32::max);
        if highest <= 0.0 {
            return None;
        }
        let (t, _) = *peaks.iter().find(|p| p.1 >= self.threshold * highest)?;

        // Parabolic interpolation around the chosen peak.
        let (a, b, c) = (self.nsdf[t - 1], self.nsdf[t], self.nsdf[t + 1]);
        let denom = a - 2.0 * b + c;
        let (shift, clarity) = if denom.abs() > 1e-12 {
            let s = 0.5 * (a - c) / denom;
            (s, b - 0.25 * (a - c) * s)
        } else {
            (0.0, b)
        };
        let period = t as f32 + shift;
        Some(PitchEstimate {
            frequency: self.sample_rate / period,
            clarity: clarity.min(1.0),
            level_db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{PluckStyle, StringVoice};

    fn cents(a: f32, b: f32) -> f32 {
        1200.0 * (a / b).log2()
    }

    #[test]
    fn detects_sine_waves() {
        let sr = 48_000.0;
        let mut d = PitchDetector::new(4096, sr);
        for f in [82.41f32, 110.0, 196.0, 440.0, 987.77] {
            let x: Vec<f32> = (0..4096)
                .map(|i| 0.3 * (std::f32::consts::TAU * f * i as f32 / sr).sin())
                .collect();
            let e = d.detect(&x).expect("pitch");
            assert!(
                cents(e.frequency, f).abs() < 1.0,
                "{f}: got {}",
                e.frequency
            );
            assert!(e.clarity > 0.95);
        }
    }

    #[test]
    fn detects_synth_guitar_strings_in_tune() {
        // Synth → detector loop: every open string of a standard-tuned guitar.
        let sr = 48_000.0;
        let mut d = PitchDetector::new(4096, sr);
        for (i, f) in [82.41f32, 110.0, 146.83, 196.0, 246.94, 329.63]
            .into_iter()
            .enumerate()
        {
            let mut v = StringVoice::new(sr, 10 + i as u32);
            v.pluck(
                f,
                PluckStyle {
                    velocity: 0.7,
                    brightness: 0.7,
                    muted: false,
                },
            );
            let x: Vec<f32> = (0..(sr as usize / 2)).map(|_| v.tick()).collect();
            let e = d.detect(&x).expect("pitch");
            assert!(
                cents(e.frequency, f).abs() < 3.0,
                "string {i} {f}: got {} ({:.1} cents)",
                e.frequency,
                cents(e.frequency, f)
            );
        }
    }

    #[test]
    fn silence_and_noise_give_no_pitch() {
        let mut d = PitchDetector::new(2048, 48_000.0);
        assert!(d.detect(&vec![0.0; 2048]).is_none());
        let mut s = 1u32;
        let noise: Vec<f32> = (0..2048)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                (s as f32 / u32::MAX as f32 - 0.5) * 0.5
            })
            .collect();
        if let Some(e) = d.detect(&noise) {
            assert!(e.clarity < 0.6, "noise should not look like a clear note");
        }
    }
}
