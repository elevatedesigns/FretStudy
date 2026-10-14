//! Tunings and the map from (string, fret) to pitch.
//!
//! **Convention used everywhere in FretStudy code:** string *index* 0 is the thickest
//! string (string 6, low E in standard tuning) and index 5 is the thinnest (string 1).
//! This matches how chord shapes are written ("x32010" = low E → high E).
//! Use [`string_number`] to convert an index to the guitarist's string number.

use crate::note::{NoteName, Pitch, PitchClass, Spelling};

pub const STRINGS: usize = 6;
/// Frets drawn on a full neck map.
pub const MAX_FRET: u8 = 22;

/// Guitarist's string number (1 = thinnest) from a string index (0 = thickest).
pub fn string_number(index: usize) -> u8 {
    (STRINGS - index) as u8
}

/// String index (0 = thickest) from a guitarist's string number (1 = thinnest).
pub fn string_index(number: u8) -> usize {
    STRINGS - number as usize
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tuning {
    pub name: &'static str,
    /// Open-string pitches from thickest (index 0) to thinnest (index 5).
    pub open: [Pitch; STRINGS],
}

impl Tuning {
    pub fn standard() -> Tuning {
        Tuning {
            name: "Standard (E A D G B E)",
            open: [40, 45, 50, 55, 59, 64].map(Pitch),
        }
    }
    pub fn drop_d() -> Tuning {
        Tuning {
            name: "Drop D (D A D G B E)",
            open: [38, 45, 50, 55, 59, 64].map(Pitch),
        }
    }
    pub fn open_d() -> Tuning {
        Tuning {
            name: "Open D (D A D F♯ A D)",
            open: [38, 45, 50, 54, 57, 62].map(Pitch),
        }
    }
    pub fn open_g() -> Tuning {
        Tuning {
            name: "Open G (D G D G B D)",
            open: [38, 43, 50, 55, 59, 62].map(Pitch),
        }
    }
    pub fn dadgad() -> Tuning {
        Tuning {
            name: "DADGAD (D A D G A D)",
            open: [38, 45, 50, 55, 57, 62].map(Pitch),
        }
    }

    pub fn pitch(&self, string: usize, fret: u8) -> Pitch {
        Pitch(self.open[string].0 + fret as i32)
    }

    /// Letter names of the open strings, thickest first (E A D G B E).
    pub fn open_names(&self) -> [NoteName; STRINGS] {
        self.open
            .map(|p| NoteName::from_pc(p.pc(), Spelling::Sharps))
    }

    /// Every (string, fret) in a fret range whose note has the given pitch class.
    pub fn positions_of(
        &self,
        value: PitchClass,
        frets: std::ops::RangeInclusive<u8>,
    ) -> Vec<(usize, u8)> {
        let mut out = Vec::new();
        for string in 0..STRINGS {
            for fret in frets.clone() {
                if self.pitch(string, fret).pc() == value % 12 {
                    out.push((string, fret));
                }
            }
        }
        out
    }

    /// The fret on `string` (lowest at or above `min_fret`) that plays `value`.
    pub fn fret_for(&self, string: usize, value: PitchClass, min_fret: u8) -> u8 {
        (min_fret..min_fret + 12)
            .find(|f| self.pitch(string, *f).pc() == value % 12)
            .expect("every pitch class appears within 12 frets")
    }

    /// For tuning by ear: the fret on `string` that should match the next thinner open string
    /// (5 in standard tuning, except 4 on the G string).
    pub fn unison_fret_for_next_string(&self, string: usize) -> Option<u8> {
        if string + 1 >= STRINGS {
            return None;
        }
        let diff = self.open[string + 1].0 - self.open[string].0;
        (diff >= 0).then_some(diff as u8)
    }
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_numbering() {
        assert_eq!(string_number(0), 6);
        assert_eq!(string_number(5), 1);
        assert_eq!(string_index(1), 5);
    }

    #[test]
    fn standard_notes() {
        let t = Tuning::standard();
        let names: Vec<String> = t.open_names().iter().map(|n| n.to_string()).collect();
        assert_eq!(names, ["E", "A", "D", "G", "B", "E"]);
        // String 6 natural notes: F1 G3 A5 B7 C8 D10 E12 (L4.1)
        for (fret, name) in [
            (1, "F"),
            (3, "G"),
            (5, "A"),
            (7, "B"),
            (8, "C"),
            (10, "D"),
            (12, "E"),
        ] {
            assert_eq!(
                NoteName::from_pc(t.pitch(0, fret).pc(), Spelling::Sharps).to_string(),
                name
            );
        }
        // String 5: B2 C3 D5 E7 F8 G10 A12 (L4.2)
        for (fret, name) in [
            (2, "B"),
            (3, "C"),
            (5, "D"),
            (7, "E"),
            (8, "F"),
            (10, "G"),
            (12, "A"),
        ] {
            assert_eq!(
                NoteName::from_pc(t.pitch(1, fret).pc(), Spelling::Sharps).to_string(),
                name
            );
        }
        // Same note five frets lower on string 5 (L4.2 shortcut)
        assert_eq!(t.pitch(0, 7), t.pitch(1, 2));
    }

    #[test]
    fn tuning_by_ear_frets() {
        // L3.10: fret 5 everywhere except the G string (fret 4)
        let t = Tuning::standard();
        let frets: Vec<u8> = (0..5)
            .map(|s| t.unison_fret_for_next_string(s).unwrap())
            .collect();
        assert_eq!(frets, [5, 5, 5, 4, 5]);
    }

    #[test]
    fn first_position_notes() {
        // L3.8: string 4: D(0) E(2) F(3); string 3: G(0) A(2); string 2: B(0) C(1) D(3)
        let t = Tuning::standard();
        let name =
            |s: usize, f: u8| NoteName::from_pc(t.pitch(s, f).pc(), Spelling::Sharps).to_string();
        assert_eq!([name(2, 0), name(2, 2), name(2, 3)], ["D", "E", "F"]);
        assert_eq!([name(3, 0), name(3, 2)], ["G", "A"]);
        assert_eq!([name(4, 0), name(4, 1), name(4, 3)], ["B", "C", "D"]);
        assert_eq!([name(5, 0), name(5, 1), name(5, 3)], ["E", "F", "G"]);
    }
}
