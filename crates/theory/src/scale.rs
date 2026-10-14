//! Scales and modes as interval recipes, spelled correctly from any root.

use crate::interval::{Interval as I, Role};
use crate::note::{NoteName, PitchClass, pc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScaleKind {
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    Chromatic,
}

impl ScaleKind {
    pub const ALL: [ScaleKind; 13] = [
        ScaleKind::Major,
        ScaleKind::NaturalMinor,
        ScaleKind::HarmonicMinor,
        ScaleKind::MelodicMinor,
        ScaleKind::Dorian,
        ScaleKind::Phrygian,
        ScaleKind::Lydian,
        ScaleKind::Mixolydian,
        ScaleKind::Locrian,
        ScaleKind::MajorPentatonic,
        ScaleKind::MinorPentatonic,
        ScaleKind::Blues,
        ScaleKind::Chromatic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ScaleKind::Major => "major",
            ScaleKind::NaturalMinor => "natural minor",
            ScaleKind::HarmonicMinor => "harmonic minor",
            ScaleKind::MelodicMinor => "melodic minor",
            ScaleKind::Dorian => "Dorian",
            ScaleKind::Phrygian => "Phrygian",
            ScaleKind::Lydian => "Lydian",
            ScaleKind::Mixolydian => "Mixolydian",
            ScaleKind::Locrian => "Locrian",
            ScaleKind::MajorPentatonic => "major pentatonic",
            ScaleKind::MinorPentatonic => "minor pentatonic",
            ScaleKind::Blues => "blues",
            ScaleKind::Chromatic => "chromatic",
        }
    }

    /// The scale's recipe as intervals from the root.
    pub fn intervals(self) -> &'static [I] {
        use ScaleKind::*;
        match self {
            Major => &[I::ROOT, I::MAJ2, I::MAJ3, I::P4, I::P5, I::MAJ6, I::MAJ7],
            NaturalMinor => &[I::ROOT, I::MAJ2, I::MIN3, I::P4, I::P5, I::MIN6, I::MIN7],
            HarmonicMinor => &[I::ROOT, I::MAJ2, I::MIN3, I::P4, I::P5, I::MIN6, I::MAJ7],
            MelodicMinor => &[I::ROOT, I::MAJ2, I::MIN3, I::P4, I::P5, I::MAJ6, I::MAJ7],
            Dorian => &[I::ROOT, I::MAJ2, I::MIN3, I::P4, I::P5, I::MAJ6, I::MIN7],
            Phrygian => &[I::ROOT, I::MIN2, I::MIN3, I::P4, I::P5, I::MIN6, I::MIN7],
            Lydian => &[I::ROOT, I::MAJ2, I::MAJ3, I::AUG4, I::P5, I::MAJ6, I::MAJ7],
            Mixolydian => &[I::ROOT, I::MAJ2, I::MAJ3, I::P4, I::P5, I::MAJ6, I::MIN7],
            Locrian => &[I::ROOT, I::MIN2, I::MIN3, I::P4, I::DIM5, I::MIN6, I::MIN7],
            MajorPentatonic => &[I::ROOT, I::MAJ2, I::MAJ3, I::P5, I::MAJ6],
            MinorPentatonic => &[I::ROOT, I::MIN3, I::P4, I::P5, I::MIN7],
            Blues => &[I::ROOT, I::MIN3, I::P4, I::DIM5, I::P5, I::MIN7],
            Chromatic => &[
                I::ROOT,
                I::MIN2,
                I::MAJ2,
                I::MIN3,
                I::MAJ3,
                I::P4,
                I::AUG4,
                I::P5,
                I::MIN6,
                I::MAJ6,
                I::MIN7,
                I::MAJ7,
            ],
        }
    }

    /// Whole/half step pattern, e.g. "W W H W W W H" for major. Pentatonic gaps show as "m3".
    pub fn step_pattern(self) -> Vec<&'static str> {
        let ints = self.intervals();
        let mut steps = Vec::new();
        for i in 0..ints.len() {
            let a = ints[i].semitones as i32;
            let b = if i + 1 < ints.len() {
                ints[i + 1].semitones as i32
            } else {
                12
            };
            steps.push(match b - a {
                1 => "H",
                2 => "W",
                3 => "m3",
                _ => "?",
            });
        }
        steps
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scale {
    pub root: NoteName,
    pub kind: ScaleKind,
}

impl Scale {
    pub fn new(root: NoteName, kind: ScaleKind) -> Self {
        Self { root, kind }
    }

    /// The scale's notes, each letter used once for 7-note scales.
    pub fn notes(&self) -> Vec<NoteName> {
        self.kind
            .intervals()
            .iter()
            .map(|i| i.above(self.root))
            .collect()
    }

    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        self.notes().iter().map(|n| n.pc()).collect()
    }

    pub fn contains(&self, value: PitchClass) -> bool {
        self.interval_of(value).is_some()
    }

    /// The interval of a pitch class above the scale's root, if it's in the scale.
    pub fn interval_of(&self, value: PitchClass) -> Option<I> {
        let distance = pc(value as i32 - self.root.pc() as i32);
        self.kind
            .intervals()
            .iter()
            .copied()
            .find(|i| i.semitones % 12 == distance)
    }

    /// The note's job for coloring: root/3rd/5th/7th/other, and blue note for the blues ♭5.
    pub fn role_of(&self, value: PitchClass) -> Option<Role> {
        let interval = self.interval_of(value)?;
        if self.kind == ScaleKind::Blues && interval == I::DIM5 {
            return Some(Role::BlueNote);
        }
        Some(interval.role())
    }

    /// Display label for a note in this scale: R, 2, ♭3, ♭5…
    pub fn label_of(&self, value: PitchClass) -> Option<String> {
        self.interval_of(value).map(|i| i.label())
    }

    /// Number of sharps (positive) or flats (negative) when the scale is written out.
    pub fn accidental_count(&self) -> i32 {
        self.notes().iter().map(|n| n.accidental as i32).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(scale: Scale) -> String {
        scale
            .notes()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn n(s: &str) -> NoteName {
        s.parse().unwrap()
    }

    #[test]
    fn major_scales_from_curriculum() {
        assert_eq!(names(Scale::new(n("C"), ScaleKind::Major)), "C D E F G A B");
        assert_eq!(
            names(Scale::new(n("G"), ScaleKind::Major)),
            "G A B C D E F#"
        );
        assert_eq!(
            names(Scale::new(n("D"), ScaleKind::Major)),
            "D E F# G A B C#"
        );
        assert_eq!(
            names(Scale::new(n("F"), ScaleKind::Major)),
            "F G A Bb C D E"
        );
        assert_eq!(
            names(Scale::new(n("Ab"), ScaleKind::Major)),
            "Ab Bb C Db Eb F G"
        );
        assert_eq!(
            names(Scale::new(n("C#"), ScaleKind::Major)),
            "C# D# E# F# G# A# B#"
        );
        assert_eq!(
            names(Scale::new(n("E"), ScaleKind::Major)),
            "E F# G# A B C# D#"
        );
    }

    #[test]
    fn step_recipe() {
        assert_eq!(ScaleKind::Major.step_pattern().join(" "), "W W H W W W H");
        assert_eq!(
            ScaleKind::NaturalMinor.step_pattern().join(" "),
            "W H W W H W W"
        );
    }

    #[test]
    fn pentatonic_and_blues() {
        assert_eq!(
            names(Scale::new(n("A"), ScaleKind::MinorPentatonic)),
            "A C D E G"
        );
        assert_eq!(
            names(Scale::new(n("C"), ScaleKind::MajorPentatonic)),
            "C D E G A"
        );
        assert_eq!(
            names(Scale::new(n("G"), ScaleKind::MajorPentatonic)),
            "G A B D E"
        );
        assert_eq!(names(Scale::new(n("A"), ScaleKind::Blues)), "A C D Eb E G");
        let blues = Scale::new(n("A"), ScaleKind::Blues);
        assert_eq!(blues.role_of(3), Some(Role::BlueNote)); // E♭
        assert_eq!(blues.role_of(9), Some(Role::Root));
        assert_eq!(blues.role_of(0), Some(Role::Third));
        assert_eq!(blues.label_of(7), Some("♭7".into()));
    }

    #[test]
    fn modes_and_minors() {
        assert_eq!(
            names(Scale::new(n("D"), ScaleKind::Dorian)),
            "D E F G A B C"
        );
        assert_eq!(
            names(Scale::new(n("A"), ScaleKind::HarmonicMinor)),
            "A B C D E F G#"
        );
        assert_eq!(
            names(Scale::new(n("F"), ScaleKind::Lydian)),
            "F G A B C D E"
        );
        assert_eq!(
            names(Scale::new(n("G"), ScaleKind::Mixolydian)),
            "G A B C D E F"
        );
        assert_eq!(
            names(Scale::new(n("B"), ScaleKind::Locrian)),
            "B C D E F G A"
        );
    }

    #[test]
    fn key_signature_counts() {
        assert_eq!(Scale::new(n("D"), ScaleKind::Major).accidental_count(), 2);
        assert_eq!(Scale::new(n("Bb"), ScaleKind::Major).accidental_count(), -2);
        assert_eq!(Scale::new(n("Db"), ScaleKind::Major).accidental_count(), -5);
    }
}
