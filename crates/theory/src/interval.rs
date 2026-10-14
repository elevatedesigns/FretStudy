//! Intervals: the distance between two notes, with both a size in half steps and a
//! letter "degree" (2nd, 3rd, 5th…), which is what makes correct spelling possible.

use crate::note::NoteName;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    /// 1 = unison/root, 3 = third, 9 = ninth, …
    pub degree: u8,
    /// Size in half steps (frets).
    pub semitones: u8,
}

/// What job a note does in a chord or scale. Drives the Note-Role colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Root,
    Third,
    Fifth,
    Seventh,
    /// 2nds/9ths, 4ths/11ths, 6ths/13ths.
    Other,
    /// The ♭5 of the blues scale.
    BlueNote,
}

impl Interval {
    pub const fn new(degree: u8, semitones: u8) -> Self {
        Self { degree, semitones }
    }

    pub const ROOT: Interval = Interval::new(1, 0);
    pub const MIN2: Interval = Interval::new(2, 1);
    pub const MAJ2: Interval = Interval::new(2, 2);
    pub const AUG2: Interval = Interval::new(2, 3);
    pub const MIN3: Interval = Interval::new(3, 3);
    pub const MAJ3: Interval = Interval::new(3, 4);
    pub const P4: Interval = Interval::new(4, 5);
    pub const AUG4: Interval = Interval::new(4, 6);
    pub const DIM5: Interval = Interval::new(5, 6);
    pub const P5: Interval = Interval::new(5, 7);
    pub const AUG5: Interval = Interval::new(5, 8);
    pub const MIN6: Interval = Interval::new(6, 8);
    pub const MAJ6: Interval = Interval::new(6, 9);
    pub const DIM7: Interval = Interval::new(7, 9);
    pub const MIN7: Interval = Interval::new(7, 10);
    pub const MAJ7: Interval = Interval::new(7, 11);
    pub const OCTAVE: Interval = Interval::new(8, 12);
    pub const MIN9: Interval = Interval::new(9, 13);
    pub const MAJ9: Interval = Interval::new(9, 14);
    pub const AUG9: Interval = Interval::new(9, 15);
    pub const P11: Interval = Interval::new(11, 17);
    pub const AUG11: Interval = Interval::new(11, 18);
    pub const MIN13: Interval = Interval::new(13, 20);
    pub const MAJ13: Interval = Interval::new(13, 21);

    /// The note this interval lands on above `root`, spelled correctly.
    pub fn above(self, root: NoteName) -> NoteName {
        root.up(self.semitones as i32, self.degree as i32 - 1)
    }

    /// Degree folded into one octave (9 → 2, 11 → 4, 13 → 6).
    pub fn simple_degree(self) -> u8 {
        (self.degree - 1) % 7 + 1
    }

    /// Short label used on diagrams: R, ♭3, 3, 5, ♭7, 9, ♯11…
    pub fn label(self) -> String {
        if self.degree == 1 && self.semitones == 0 {
            return "R".to_string();
        }
        let natural = natural_semitones(self.degree);
        let diff = self.semitones as i32 - natural;
        let prefix = match diff {
            0 => String::new(),
            d if d > 0 => "♯".repeat(d as usize),
            d => "♭".repeat((-d) as usize),
        };
        format!("{}{}", prefix, self.degree)
    }

    /// Full quality name, e.g. "major 3rd", "perfect 5th".
    pub fn name(self) -> String {
        let d = self.simple_degree();
        let natural = natural_semitones(self.degree);
        let diff = self.semitones as i32 - natural;
        let perfect = matches!(d, 1 | 4 | 5);
        let quality = match (perfect, diff) {
            (true, 0) => "perfect",
            (true, -1) => "diminished",
            (true, 1) => "augmented",
            (false, 0) => "major",
            (false, -1) => "minor",
            (false, -2) => "diminished",
            (false, 1) => "augmented",
            _ => "altered",
        };
        if self.degree == 1 {
            return if diff == 0 {
                "unison".into()
            } else {
                format!("{quality} unison")
            };
        }
        if self.degree == 8 && diff == 0 {
            return "octave".into();
        }
        format!("{quality} {}", ordinal(self.degree))
    }

    pub fn role(self) -> Role {
        match self.simple_degree() {
            1 => Role::Root,
            3 => Role::Third,
            5 => Role::Fifth,
            7 => Role::Seventh,
            _ => Role::Other,
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// Half steps of the major-scale version of a degree (2 → 2, 3 → 4, 9 → 14…).
fn natural_semitones(degree: u8) -> i32 {
    let simple = (degree - 1) % 7;
    let octaves = (degree - 1) / 7;
    [0, 2, 4, 5, 7, 9, 11][simple as usize] + 12 * octaves as i32
}

fn ordinal(n: u8) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Plain-language interval name from a number of half steps within an octave,
/// as taught in the Ear Lab lessons.
pub fn simple_interval_name(semitones: i32) -> &'static str {
    match semitones.rem_euclid(12) {
        0 => "unison / octave",
        1 => "minor 2nd",
        2 => "major 2nd",
        3 => "minor 3rd",
        4 => "major 3rd",
        5 => "perfect 4th",
        6 => "tritone",
        7 => "perfect 5th",
        8 => "minor 6th",
        9 => "major 6th",
        10 => "minor 7th",
        _ => "major 7th",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_names() {
        assert_eq!(Interval::ROOT.label(), "R");
        assert_eq!(Interval::MIN3.label(), "♭3");
        assert_eq!(Interval::MAJ3.label(), "3");
        assert_eq!(Interval::DIM5.label(), "♭5");
        assert_eq!(Interval::MIN7.label(), "♭7");
        assert_eq!(Interval::DIM7.label(), "♭♭7");
        assert_eq!(Interval::AUG11.label(), "♯11");
        assert_eq!(Interval::MAJ3.name(), "major 3rd");
        assert_eq!(Interval::P5.name(), "perfect 5th");
        assert_eq!(Interval::DIM5.name(), "diminished 5th");
        assert_eq!(Interval::AUG2.name(), "augmented 2nd");
        assert_eq!(Interval::OCTAVE.name(), "octave");
        assert_eq!(Interval::MAJ13.name(), "major 13th");
    }

    #[test]
    fn spelled_above() {
        let c: NoteName = "C".parse().unwrap();
        assert_eq!(Interval::MIN3.above(c).to_string(), "Eb");
        assert_eq!(Interval::MIN6.above(c).to_string(), "Ab");
        let e: NoteName = "E".parse().unwrap();
        assert_eq!(Interval::AUG2.above(e).to_string(), "F##");
        let bb: NoteName = "Bb".parse().unwrap();
        assert_eq!(Interval::DIM5.above(bb).to_string(), "Fb");
    }

    #[test]
    fn roles() {
        assert_eq!(Interval::MAJ9.role(), Role::Other);
        assert_eq!(Interval::MIN7.role(), Role::Seventh);
        assert_eq!(Interval::DIM5.role(), Role::Fifth);
    }
}
