//! Reading and writing simple WAV files (mono, 16-bit PCM) for the learner's recordings.

use anyhow::{Context, Result, bail};
use std::io::{Read, Write};
use std::path::Path;

/// Writes mono samples (-1..1) as a 16-bit WAV file.
pub fn write(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    let mut f =
        std::fs::File::create(path).with_context(|| format!("Couldn't save {}", path.display()))?;
    f.write_all(&out)?;
    Ok(())
}

/// Reads a 16-bit PCM WAV (mono or stereo, mixed to mono). Returns (samples, sample rate).
pub fn read(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .with_context(|| format!("Couldn't open {}", path.display()))?
        .read_to_end(&mut bytes)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{} isn't a WAV file", path.display());
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let mut pos = 12;
    let (mut channels, mut rate, mut bits) = (1u16, 44_100u32, 16u16);
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32_at(pos + 4) as usize;
        let body = pos + 8;
        if id == b"fmt " && body + 16 <= bytes.len() {
            channels = u16_at(body + 2).max(1);
            rate = u32_at(body + 4);
            bits = u16_at(body + 14);
        } else if id == b"data" {
            if bits != 16 {
                bail!("Only 16-bit WAV files can be played");
            }
            let end = (body + len).min(bytes.len());
            let frame = channels as usize * 2;
            let samples = bytes[body..end]
                .chunks_exact(frame)
                .map(|f| {
                    let sum: f32 = f
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| i16::from_le_bytes(*c) as f32 / i16::MAX as f32)
                        .sum();
                    sum / channels as f32
                })
                .collect();
            return Ok((samples, rate));
        }
        pos = body + len + (len & 1);
    }
    bail!("{} has no sound data", path.display())
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("fretstudy-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        let samples: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        super::write(&path, &samples, 48_000).unwrap();
        let (back, rate) = super::read(&path).unwrap();
        assert_eq!(rate, 48_000);
        assert_eq!(back.len(), samples.len());
        assert!(back.iter().zip(&samples).all(|(a, b)| (a - b).abs() < 1e-3));
        std::fs::remove_dir_all(dir).ok();
    }
}
