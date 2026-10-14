//! Keys, the chords that belong to them, and the Number System (1–4–5, 1–5–6–4…).

use crate::chord::{Chord, ChordQuality};
use crate::note::NoteName;
use crate::scale::{Scale, ScaleKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub tonic: NoteName,
    pub mode: Mode,
}

/// One chord of a key, with its jersey number and Roman numeral.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiatonicChord {
    pub number: u8,
    pub numeral: String,
    pub chord: Chord,
}

impl Key {
    pub fn major(tonic: NoteName) -> Key {
        Key {
            tonic,
            mode: Mode::Major,
        }
    }

    pub fn minor(tonic: NoteName) -> Key {
        Key {
            tonic,
            mode: Mode::Minor,
        }
    }

    pub fn scale(&self) -> Scale {
        let kind = match self.mode {
            Mode::Major => ScaleKind::Major,
            Mode::Minor => ScaleKind::NaturalMinor,
        };
        Scale::new(self.tonic, kind)
    }

    /// Relative minor (6th degree) of a major key, or relative major (3rd degree) of a minor key.
    pub fn relative(&self) -> Key {
        let notes = self.scale().notes();
        match self.mode {
            Mode::Major => Key::minor(notes[5]),
            Mode::Minor => Key::major(notes[2]),
        }
    }

    /// The 7 triads built by stacking every other scale note (1-3-5).
    pub fn triads(&self) -> Vec<DiatonicChord> {
        self.chords(false)
    }

    /// The 7 four-note chords (1-3-5-7): Imaj7 ii7 iii7 IVmaj7 V7 vi7 viiø7 in major.
    pub fn sevenths(&self) -> Vec<DiatonicChord> {
        self.chords(true)
    }

    fn chords(&self, sevenths: bool) -> Vec<DiatonicChord> {
        let notes = self.scale().notes();
        let pcs: Vec<i32> = notes.iter().map(|n| n.pc() as i32).collect();
        (0..7)
            .map(|i| {
                let span = |steps: usize| (pcs[(i + steps) % 7] - pcs[i]).rem_euclid(12);
                let (third, fifth, seventh) = (span(2), span(4), span(6));
                let quality = match (third, fifth, sevenths.then_some(seventh)) {
                    (4, 7, None) => ChordQuality::Major,
                    (3, 7, None) => ChordQuality::Minor,
                    (3, 6, None) => ChordQuality::Dim,
                    (4, 8, None) => ChordQuality::Aug,
                    (4, 7, Some(11)) => ChordQuality::Maj7,
                    (4, 7, Some(10)) => ChordQuality::Dom7,
                    (3, 7, Some(10)) => ChordQuality::Min7,
                    (3, 6, Some(10)) => ChordQuality::HalfDim7,
                    (3, 6, Some(9)) => ChordQuality::Dim7,
                    (3, 7, Some(11)) => ChordQuality::MinMaj7,
                    other => panic!("unexpected diatonic chord {other:?}"),
                };
                let number = i as u8 + 1;
                DiatonicChord {
                    number,
                    numeral: numeral(number, quality, self.mode, &pcs, i),
                    chord: Chord::new(notes[i], quality),
                }
            })
            .collect()
    }

    /// The chord with a given jersey number (1–7).
    pub fn chord_number(&self, number: u8) -> Chord {
        self.triads()[(number as usize - 1) % 7].chord
    }

    /// A progression of numbers → chords, e.g. [1, 5, 6, 4] in G → G D Em C.
    pub fn progression(&self, numbers: &[u8]) -> Vec<Chord> {
        numbers.iter().map(|n| self.chord_number(*n)).collect()
    }

    /// Sharps (positive) or flats (negative) in the key signature.
    pub fn signature(&self) -> i32 {
        let major = match self.mode {
            Mode::Major => *self,
            Mode::Minor => self.relative(),
        };
        major.scale().accidental_count()
    }

    pub fn name(&self) -> String {
        match self.mode {
            Mode::Major => format!("{} major", self.tonic.pretty()),
            Mode::Minor => format!("{} minor", self.tonic.pretty()),
        }
    }
}

fn numeral(number: u8, quality: ChordQuality, mode: Mode, pcs: &[i32], i: usize) -> String {
    const UPPER: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
    let base = UPPER[number as usize - 1];
    let minorish = matches!(
        quality,
        ChordQuality::Minor
            | ChordQuality::Min7
            | ChordQuality::Dim
            | ChordQuality::HalfDim7
            | ChordQuality::Dim7
            | ChordQuality::MinMaj7
    );
    let mut s = String::new();
    // In minor keys, degrees 3, 6, 7 sit a half step lower than in major: ♭III ♭VI ♭VII.
    if mode == Mode::Minor {
        let major_degree = [0, 2, 4, 5, 7, 9, 11][i];
        if (pcs[i] - pcs[0]).rem_euclid(12) < major_degree {
            s.push('♭');
        }
    }
    s.push_str(&if minorish {
        base.to_lowercase()
    } else {
        base.to_string()
    });
    s.push_str(match quality {
        ChordQuality::Dim => "°",
        ChordQuality::Maj7 => "maj7",
        ChordQuality::Dom7 | ChordQuality::Min7 => "7",
        ChordQuality::HalfDim7 => "ø7",
        ChordQuality::Dim7 => "°7",
        _ => "",
    });
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> NoteName {
        s.parse().unwrap()
    }

    fn chord_names(chords: &[Chord]) -> String {
        chords
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn number_system() {
        // L4.12
        let c = Key::major(n("C"));
        let names: Vec<String> = c.triads().iter().map(|d| d.chord.to_string()).collect();
        assert_eq!(names.join(" "), "C Dm Em F G Am Bdim");
        let numerals: Vec<String> = c.triads().iter().map(|d| d.numeral.clone()).collect();
        assert_eq!(numerals.join(" "), "I ii iii IV V vi vii°");
        assert_eq!(
            chord_names(&Key::major(n("G")).progression(&[1, 5, 6, 4])),
            "G D Em C"
        );
        assert_eq!(
            chord_names(&Key::major(n("D")).progression(&[1, 4, 5])),
            "D G A"
        );
        assert_eq!(Key::major(n("D")).chord_number(6).to_string(), "Bm");
        assert_eq!(
            chord_names(&Key::major(n("A")).progression(&[1, 4, 5])),
            "A D E"
        );
        // Canon in D (L6.13): 1–5–6–3–4–1–4–5
        assert_eq!(
            chord_names(&Key::major(n("D")).progression(&[1, 5, 6, 3, 4, 1, 4, 5])),
            "D A Bm F#m G D G A"
        );
    }

    #[test]
    fn sevenths() {
        // L7.1
        let c = Key::major(n("C"));
        let names: Vec<String> = c.sevenths().iter().map(|d| d.chord.to_string()).collect();
        assert_eq!(names.join(" "), "Cmaj7 Dm7 Em7 Fmaj7 G7 Am7 Bm7b5");
        let numerals: Vec<String> = c.sevenths().iter().map(|d| d.numeral.clone()).collect();
        assert_eq!(numerals.join(" "), "Imaj7 ii7 iii7 IVmaj7 V7 vi7 viiø7");
    }

    #[test]
    fn minor_keys() {
        // L7.6
        let am = Key::minor(n("A"));
        let names: Vec<String> = am.triads().iter().map(|d| d.chord.to_string()).collect();
        assert_eq!(names.join(" "), "Am Bdim C Dm Em F G");
        let numerals: Vec<String> = am.triads().iter().map(|d| d.numeral.clone()).collect();
        assert_eq!(numerals.join(" "), "i ii° ♭III iv v ♭VI ♭VII");
    }

    #[test]
    fn relatives_and_signatures() {
        // L6.1
        for (major, minor) in [
            ("C", "A"),
            ("G", "E"),
            ("D", "B"),
            ("A", "F#"),
            ("E", "C#"),
            ("F", "D"),
        ] {
            assert_eq!(Key::major(n(major)).relative().tonic.to_string(), minor);
        }
        // L4.11
        assert_eq!(Key::major(n("G")).signature(), 1);
        assert_eq!(Key::major(n("E")).signature(), 4);
        assert_eq!(Key::major(n("F")).signature(), -1);
        assert_eq!(Key::minor(n("A")).signature(), 0);
        assert_eq!(Key::minor(n("E")).signature(), 1);
    }
}
