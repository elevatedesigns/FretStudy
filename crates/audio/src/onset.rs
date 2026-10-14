//! Onset detection: finds the moment each new sound starts (a pluck, strum, or click).
//! Used for latency calibration now, and for rhythm checks later.

/// Streaming onset detector based on jumps in short-term energy.
pub struct OnsetDetector {
    frame: usize,
    acc: f32,
    acc_len: usize,
    /// Slowly moving background level.
    background: f32,
    prev_energy: f32,
    refractory: usize,
    since_onset: usize,
    position: u64,
    frame_start: u64,
    first_loud: Option<u64>,
    /// How many times louder than the background a frame must be.
    pub ratio: f32,
    /// Absolute floor (mean square) so room noise never triggers.
    pub floor: f32,
}

impl OnsetDetector {
    pub fn new(sample_rate: u32) -> Self {
        let frame = (sample_rate as usize / 375).max(64); // ~2.7 ms frames
        OnsetDetector {
            frame,
            acc: 0.0,
            acc_len: 0,
            background: 0.0,
            prev_energy: 0.0,
            refractory: (sample_rate as usize / 12) / frame, // ~83 ms between onsets
            since_onset: usize::MAX / 2,
            position: 0,
            frame_start: 0,
            first_loud: None,
            ratio: 6.0,
            floor: 1e-5,
        }
    }

    /// Feeds samples whose first sample has absolute index `start`. Returns onset sample indexes.
    pub fn push(&mut self, start: u64, samples: &[f32]) -> Vec<u64> {
        let mut onsets = Vec::new();
        if self.position != start {
            // Gap or first call: resync without inventing onsets.
            self.position = start;
            self.frame_start = start;
            self.acc = 0.0;
            self.acc_len = 0;
            self.first_loud = None;
        }
        for &s in samples {
            let sq = s * s;
            if self.first_loud.is_none() && sq > (self.background * self.ratio).max(self.floor) {
                self.first_loud = Some(self.position);
            }
            self.acc += sq;
            self.acc_len += 1;
            self.position += 1;
            if self.acc_len == self.frame {
                let energy = self.acc / self.frame as f32;
                self.since_onset = self.since_onset.saturating_add(1);
                let rising = energy > self.prev_energy * 1.5;
                if energy > self.floor
                    && energy > self.background * self.ratio
                    && rising
                    && self.since_onset > self.refractory
                {
                    onsets.push(self.first_loud.unwrap_or(self.frame_start));
                    self.since_onset = 0;
                }
                // Background follows quiet passages quickly and loud ones slowly.
                let rate = if energy < self.background { 0.2 } else { 0.01 };
                self.background += rate * (energy - self.background);
                self.prev_energy = energy;
                self.acc = 0.0;
                self.acc_len = 0;
                self.frame_start = self.position;
                self.first_loud = None;
            }
        }
        onsets
    }
}

/// Median offset (ns) between expected event times and detected onsets, pairing each
/// expected time with the nearest onset within ±`window_ns`. Needs at least `min_pairs`.
pub fn median_offset(
    expected_ns: &[u64],
    detected_ns: &[u64],
    window_ns: u64,
    min_pairs: usize,
) -> Option<i64> {
    let mut offsets: Vec<i64> = expected_ns
        .iter()
        .filter_map(|&e| {
            detected_ns
                .iter()
                .map(|&d| d as i64 - e as i64)
                .filter(|o| o.unsigned_abs() <= window_ns)
                .min_by_key(|o| o.unsigned_abs())
        })
        .collect();
    if offsets.len() < min_pairs {
        return None;
    }
    offsets.sort_unstable();
    Some(offsets[offsets.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::Click;

    #[test]
    fn finds_clicks_in_noise() {
        let sr = 48_000u32;
        let mut click = Click::new(sr as f32);
        let mut x = vec![0.0f32; sr as usize * 3];
        let mut noise = 7u32;
        let times = [12_000usize, 36_000, 60_000, 84_000, 108_000];
        for (i, s) in x.iter_mut().enumerate() {
            if times.contains(&i) {
                click.trigger(true, 0.8);
            }
            noise ^= noise << 13;
            noise ^= noise >> 17;
            noise ^= noise << 5;
            *s = click.tick() + (noise as f32 / u32::MAX as f32 - 0.5) * 0.002;
        }
        let mut d = OnsetDetector::new(sr);
        let mut found = Vec::new();
        for (i, chunk) in x.chunks(512).enumerate() {
            found.extend(d.push((i * 512) as u64, chunk));
        }
        assert_eq!(found.len(), times.len(), "{found:?}");
        for (f, t) in found.iter().zip(times) {
            assert!(
                (*f as i64 - t as i64).abs() < 200,
                "found {f}, expected {t}"
            );
        }
    }

    #[test]
    fn median_offset_pairs_nearest() {
        let expected = [
            1_000_000_000u64,
            1_500_000_000,
            2_000_000_000,
            2_500_000_000,
        ];
        let detected = [
            1_042_000_000u64,
            1_540_000_000,
            2_041_000_000,
            2_543_000_000,
            2_900_000_000,
        ];
        let m = median_offset(&expected, &detected, 200_000_000, 3).unwrap();
        assert!((m - 42_000_000).abs() <= 1_000_000);
        assert!(median_offset(&expected, &[], 200_000_000, 3).is_none());
    }
}
