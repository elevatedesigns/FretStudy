//! Turns musical ideas (a chord shape, a fretted note) into timed synth events.

use crate::synth::{PluckStyle, SynthEvent};
use theory::{ChordShape, Tuning};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrumDirection {
    /// Thick strings first (toward the floor).
    Down,
    /// Thin strings first. Up-strums usually catch only the top few strings.
    Up,
}

/// A single fretted or open note.
pub fn note(tuning: &Tuning, string: usize, fret: u8, style: PluckStyle) -> SynthEvent {
    SynthEvent::Pluck {
        string,
        freq: tuning.pitch(string, fret).frequency() as f32,
        style,
    }
}

/// Events for strumming a chord shape, starting at frame `at`, with `spread` frames between
/// strings (~8–25 ms sounds natural). Muted strings (x) are silenced, not played.
pub fn strum(
    shape: &ChordShape,
    tuning: &Tuning,
    at: u64,
    direction: StrumDirection,
    spread: u64,
    style: PluckStyle,
) -> Vec<(u64, SynthEvent)> {
    let mut strings: Vec<usize> = (0..shape.frets.len())
        .filter(|&s| shape.frets[s].is_some())
        .collect();
    let mut events: Vec<(u64, SynthEvent)> = (0..shape.frets.len())
        .filter(|&s| shape.frets[s].is_none())
        .map(|s| (at, SynthEvent::Damp { string: s }))
        .collect();
    if direction == StrumDirection::Up {
        strings.reverse();
        strings.truncate(4);
    }
    for (i, &s) in strings.iter().enumerate() {
        let fret = shape.frets[s].unwrap();
        // Later strings in a strum are a touch softer, like a real pick losing energy.
        let v = style.velocity * (1.0 - 0.04 * i as f32);
        events.push((
            at + spread * i as u64,
            note(
                tuning,
                s,
                fret,
                PluckStyle {
                    velocity: v,
                    ..style
                },
            ),
        ));
    }
    events
}

/// Events for playing a chord one string at a time (an arpeggio / "string check").
pub fn arpeggio(
    shape: &ChordShape,
    tuning: &Tuning,
    at: u64,
    gap: u64,
    style: PluckStyle,
) -> Vec<(u64, SynthEvent)> {
    (0..shape.frets.len())
        .filter_map(|s| shape.frets[s].map(|f| (s, f)))
        .enumerate()
        .map(|(i, (s, f))| (at + gap * i as u64, note(tuning, s, f, style)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strum_order_and_mutes() {
        let t = Tuning::standard();
        let c = ChordShape::parse("x32010", None).unwrap();
        let down = strum(&c, &t, 100, StrumDirection::Down, 10, PluckStyle::default());
        // One damp for the muted 6th string, then 5 plucks low → high.
        assert_eq!(
            down.iter()
                .filter(|(_, e)| matches!(e, SynthEvent::Damp { .. }))
                .count(),
            1
        );
        let plucks: Vec<(u64, usize)> = down
            .iter()
            .filter_map(|(t, e)| match e {
                SynthEvent::Pluck { string, .. } => Some((*t, *string)),
                _ => None,
            })
            .collect();
        assert_eq!(
            plucks,
            vec![(100, 1), (110, 2), (120, 3), (130, 4), (140, 5)]
        );
        let up = strum(&c, &t, 0, StrumDirection::Up, 10, PluckStyle::default());
        let first_up = up.iter().find_map(|(_, e)| match e {
            SynthEvent::Pluck { string, .. } => Some(*string),
            _ => None,
        });
        assert_eq!(first_up, Some(5));
    }
}
