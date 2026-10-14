//! Renders a short demo of the guitar sound to a WAV file, for listening to synth changes:
//! G, C, D, and G strummed, then a little melody.
//! Run: cargo run -p audio --example sound_demo -- dist/sound-demo.wav
use audio::play::{self, StrumDirection};
use audio::synth::{PluckStyle, Synth};
use theory::{ChordShape, Tuning};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sound-demo.wav".into());
    let sr = 48_000u32;
    let t = Tuning::standard();
    let mut events = Vec::new();
    let beat = (sr as f64 * 0.5) as u64; // 120 BPM
    let chords = ["320003", "x32010", "xx0232", "320003"];
    for (i, frets) in chords.iter().enumerate() {
        let shape = ChordShape::parse(frets, None).unwrap();
        for b in 0..4u64 {
            let at = (i as u64 * 4 + b) * beat + sr as u64 / 10;
            let dir = if b % 2 == 0 {
                StrumDirection::Down
            } else {
                StrumDirection::Up
            };
            let vel = if b == 0 { 0.85 } else { 0.6 };
            events.extend(play::strum(
                &shape,
                &t,
                at,
                dir,
                sr as u64 / 70,
                PluckStyle {
                    velocity: vel,
                    ..Default::default()
                },
            ));
        }
    }
    // Melody: Ode to Joy on strings 1-2.
    let melody = [
        (1, 0),
        (1, 0),
        (1, 1),
        (1, 3),
        (1, 3),
        (1, 1),
        (1, 0),
        (2, 3),
        (2, 1),
        (2, 1),
        (2, 3),
        (1, 0),
        (1, 0),
        (2, 3),
        (2, 3),
    ];
    let start = 17 * beat;
    for (k, (string, fret)) in melody.iter().enumerate() {
        events.push((
            start + k as u64 * beat,
            play::note(&t, 6 - *string as usize, *fret, PluckStyle::default()),
        ));
    }
    events.sort_by_key(|e| e.0);
    let total = start + melody.len() as u64 * beat + 2 * sr as u64;
    let mut synth = Synth::new(sr as f32);
    let mut samples = Vec::with_capacity(total as usize);
    let mut next = 0;
    for i in 0..total {
        while next < events.len() && events[next].0 <= i {
            synth.handle(events[next].1);
            next += 1;
        }
        samples.push(synth.tick());
    }
    let peak = samples.iter().fold(0.0f32, |a, x| a.max(x.abs()));
    audio::wav::write(std::path::Path::new(&out), &samples, sr).unwrap();
    println!(
        "Wrote {out} ({:.1} s, peak {peak:.2})",
        total as f32 / sr as f32
    );
}
