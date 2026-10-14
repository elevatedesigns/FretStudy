//! How a take was played: how loud the notes were, and how long they rang (for the
//! expression lesson: soft vs loud, legato vs staccato).

/// Loudness and note-length summary of one take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TakeStats {
    /// How loud the notes are at their loudest (dBFS, 95th percentile of 10 ms levels).
    pub attack_db: f32,
    /// Share of the playing time the sound stays within 20 dB of its attack (0..1).
    /// Long, ringing notes score high; short, choked notes score low.
    pub sustain: f32,
}

/// Measures a take (mono samples). Returns None if almost nothing was played.
pub fn measure(samples: &[f32], sample_rate: u32) -> Option<TakeStats> {
    let win = (sample_rate as usize / 100).max(1); // 10 ms windows
    let levels: Vec<f32> = samples
        .chunks(win)
        .map(|c| {
            let rms = (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt();
            20.0 * rms.max(1e-6).log10()
        })
        .collect();
    let loudest = levels.iter().copied().fold(f32::MIN, f32::max);
    if loudest < -50.0 {
        return None;
    }
    // Only the part where something is being played (after the first loud moment).
    let start = levels.iter().position(|l| *l > loudest - 30.0)?;
    let end = levels.iter().rposition(|l| *l > loudest - 30.0)? + 1;
    let span = &levels[start..end];
    let mut sorted = span.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    // The loud end of the take stands for how hard the notes were played.
    let attack_db = sorted[sorted.len() * 95 / 100];
    let ringing = span.iter().filter(|l| **l > attack_db - 20.0).count();
    Some(TakeStats {
        attack_db,
        sustain: ringing as f32 / span.len() as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{PluckStyle, StringVoice};

    fn take(velocity: f32, choke_after: Option<usize>) -> Vec<f32> {
        let sr = 48_000usize;
        let mut v = StringVoice::new(sr as f32, 9);
        let mut out = Vec::new();
        for _note in 0..6 {
            v.pluck(
                220.0,
                PluckStyle {
                    velocity,
                    brightness: 0.5,
                    muted: false,
                },
            );
            for i in 0..sr / 2 {
                if choke_after == Some(i) {
                    v.damp();
                }
                out.push(v.tick());
            }
        }
        out
    }

    #[test]
    fn tells_soft_from_loud_and_short_from_long() {
        let soft = measure(&take(0.2, None), 48_000).unwrap();
        let loud = measure(&take(0.9, None), 48_000).unwrap();
        let short = measure(&take(0.9, Some(4_000)), 48_000).unwrap();
        assert!(loud.attack_db - soft.attack_db > 6.0, "{soft:?} {loud:?}");
        assert!(loud.sustain - short.sustain > 0.3, "{loud:?} {short:?}");
    }
}
