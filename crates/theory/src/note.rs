//! Note letters, spelled note names, pitch classes, and concrete pitches.

use std::fmt;
use std::str::FromStr;

/// A pitch class: 0 = C, 1 = C♯/D♭, … 11 = B.
pub type PitchClass = u8;

/// Wraps any integer into the 0..12 pitch-class range.
pub fn pc(value: i32) -> PitchClass {
    value.rem_euclid(12) as PitchClass
}

/// One of the seven musical-alphabet letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Letter {
    pub const ALL: [Letter; 7] = [
        Letter::C,
        Letter::D,
        Letter::E,
        Letter::F,
        Letter::G,
        Letter::A,
        Letter::B,
    ];

    /// Position in the alphabet starting from C (C = 0 … B = 6).
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: i32) -> Letter {
        Letter::ALL[index.rem_euclid(7) as usize]
    }

    /// Pitch class of the natural (no sharp or flat) note.
    pub fn natural_pc(self) -> PitchClass {
        [0, 2, 4, 5, 7, 9, 11][self.index()]
    }

    /// The letter `steps` letters above this one (wrapping G → A).
    pub fn up(self, steps: i32) -> Letter {
        Letter::from_index(self.index() as i32 + steps)
    }

    pub fn as_char(self) -> char {
        ['C', 'D', 'E', 'F', 'G', 'A', 'B'][self.index()]
    }
}

/// A spelled note name such as C, F♯, or B♭. Spelling matters: A♯ and B♭ sound the
/// same but are different names, and scales use each letter exactly once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoteName {
    pub letter: Letter,
    /// Sharps (positive) or flats (negative). Usually -2..=2.
    pub accidental: i8,
}

/// Which accidental to prefer when a name has to be invented from a pitch class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    Sharps,
    Flats,
}

impl NoteName {
    pub const fn new(letter: Letter, accidental: i8) -> Self {
        Self { letter, accidental }
    }

    pub const fn natural(letter: Letter) -> Self {
        Self::new(letter, 0)
    }

    pub fn pc(self) -> PitchClass {
        pc(self.letter.natural_pc() as i32 + self.accidental as i32)
    }

    /// Names a pitch class using naturals where possible, otherwise the preferred accidental.
    pub fn from_pc(value: PitchClass, spelling: Spelling) -> NoteName {
        let value = value % 12;
        if let Some(letter) = Letter::ALL.iter().find(|l| l.natural_pc() == value) {
            return NoteName::natural(*letter);
        }
        match spelling {
            Spelling::Sharps => {
                let letter = *Letter::ALL
                    .iter()
                    .find(|l| l.natural_pc() == pc(value as i32 - 1))
                    .unwrap();
                NoteName::new(letter, 1)
            }
            Spelling::Flats => {
                let letter = *Letter::ALL
                    .iter()
                    .find(|l| l.natural_pc() == pc(value as i32 + 1))
                    .unwrap();
                NoteName::new(letter, -1)
            }
        }
    }

    /// Names the note `semitones` above, landing on the letter `letter_steps` above.
    /// For example C up (4 semitones, 2 letters) = E; D up (4, 2) = F♯.
    pub fn up(self, semitones: i32, letter_steps: i32) -> NoteName {
        let letter = self.letter.up(letter_steps);
        let target = pc(self.pc() as i32 + semitones) as i32;
        let mut diff = target - letter.natural_pc() as i32;
        if diff > 6 {
            diff -= 12;
        }
        if diff < -6 {
            diff += 12;
        }
        NoteName::new(letter, diff as i8)
    }

    /// Moves by semitones and re-spells from the pitch class.
    pub fn transpose(self, semitones: i32, spelling: Spelling) -> NoteName {
        NoteName::from_pc(pc(self.pc() as i32 + semitones), spelling)
    }

    /// The same sound under its simplest name (for example E♯ → F, C♭ → B).
    pub fn simplified(self, spelling: Spelling) -> NoteName {
        if (-1..=1).contains(&self.accidental)
            && !matches!(
                (self.letter, self.accidental),
                (Letter::E, 1) | (Letter::B, 1) | (Letter::F, -1) | (Letter::C, -1)
            )
        {
            self
        } else {
            NoteName::from_pc(self.pc(), spelling)
        }
    }

    /// Pretty name using ♯ ♭ 𝄪 𝄫 symbols.
    pub fn pretty(self) -> String {
        let mut s = String::new();
        s.push(self.letter.as_char());
        match self.accidental {
            0 => {}
            1 => s.push('♯'),
            2 => s.push('𝄪'),
            -1 => s.push('♭'),
            -2 => s.push('𝄫'),
            n if n > 0 => s.push_str(&"♯".repeat(n as usize)),
            n => s.push_str(&"♭".repeat((-n) as usize)),
        }
        s
    }
}

impl fmt::Display for NoteName {
    /// ASCII spelling: C, C#, Bb, F##, Ebb.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter.as_char())?;
        if self.accidental > 0 {
            write!(f, "{}", "#".repeat(self.accidental as usize))
        } else {
            write!(f, "{}", "b".repeat((-self.accidental) as usize))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNoteError(pub String);

impl fmt::Display for ParseNoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a note name: {:?}", self.0)
    }
}

impl std::error::Error for ParseNoteError {}

impl NoteName {
    /// Parses a note name from the start of `s`, returning it and the rest of the string.
    /// Accepts `#`, `b`, `♯`, `♭`, `𝄪`, `𝄫`.
    pub fn parse_prefix(s: &str) -> Result<(NoteName, &str), ParseNoteError> {
        let mut chars = s.char_indices();
        let (_, first) = chars.next().ok_or_else(|| ParseNoteError(s.to_string()))?;
        let letter = match first.to_ascii_uppercase() {
            'C' => Letter::C,
            'D' => Letter::D,
            'E' => Letter::E,
            'F' => Letter::F,
            'G' => Letter::G,
            'A' => Letter::A,
            'B' => Letter::B,
            _ => return Err(ParseNoteError(s.to_string())),
        };
        let mut accidental = 0i8;
        let mut rest = &s[first.len_utf8()..];
        while let Some(c) = rest.chars().next() {
            let delta = match c {
                '#' | '♯' => 1,
                'b' | '♭' => -1,
                '𝄪' => 2,
                '𝄫' => -2,
                _ => break,
            };
            accidental += delta;
            rest = &rest[c.len_utf8()..];
        }
        Ok((NoteName::new(letter, accidental), rest))
    }
}

impl FromStr for NoteName {
    type Err = ParseNoteError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (note, rest) = NoteName::parse_prefix(s.trim())?;
        if rest.is_empty() {
            Ok(note)
        } else {
            Err(ParseNoteError(s.to_string()))
        }
    }
}

/// A concrete pitch as a MIDI note number (middle C = C4 = 60, low E string = E2 = 40).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pitch(pub i32);

impl Pitch {
    pub const A4_HZ: f64 = 440.0;

    pub fn pc(self) -> PitchClass {
        pc(self.0)
    }

    /// Scientific octave number (C4 = middle C).
    pub fn octave(self) -> i32 {
        self.0.div_euclid(12) - 1
    }

    pub fn frequency(self) -> f64 {
        Self::A4_HZ * 2f64.powf((self.0 as f64 - 69.0) / 12.0)
    }

    /// Nearest pitch to a frequency, plus how far off it is in cents (−50..50).
    pub fn nearest(frequency_hz: f64) -> (Pitch, f64) {
        let midi = 69.0 + 12.0 * (frequency_hz / Self::A4_HZ).log2();
        let rounded = midi.round();
        (Pitch(rounded as i32), (midi - rounded) * 100.0)
    }

    /// Cents from this pitch to a frequency (positive = sharp).
    pub fn cents_to(self, frequency_hz: f64) -> f64 {
        1200.0 * (frequency_hz / self.frequency()).log2()
    }

    pub fn name(self, spelling: Spelling) -> String {
        format!(
            "{}{}",
            NoteName::from_pc(self.pc(), spelling),
            self.octave()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_classes() {
        assert_eq!("C".parse::<NoteName>().unwrap().pc(), 0);
        assert_eq!("F#".parse::<NoteName>().unwrap().pc(), 6);
        assert_eq!("Gb".parse::<NoteName>().unwrap().pc(), 6);
        assert_eq!("B♭".parse::<NoteName>().unwrap().pc(), 10);
        assert_eq!("Cb".parse::<NoteName>().unwrap().pc(), 11);
        assert_eq!("E#".parse::<NoteName>().unwrap().pc(), 5);
        assert!("H".parse::<NoteName>().is_err());
        assert!("C#x".parse::<NoteName>().is_err());
    }

    #[test]
    fn spelling_up_by_letter() {
        let c: NoteName = "C".parse().unwrap();
        assert_eq!(c.up(4, 2).to_string(), "E");
        let d: NoteName = "D".parse().unwrap();
        assert_eq!(d.up(4, 2).to_string(), "F#");
        let f: NoteName = "F".parse().unwrap();
        assert_eq!(f.up(5, 3).to_string(), "Bb"); // a perfect 4th above F is B♭, not A♯
        let gb: NoteName = "Gb".parse().unwrap();
        assert_eq!(gb.up(3, 2).to_string(), "Bbb"); // Learn & Master's double-flat example
        let e: NoteName = "E".parse().unwrap();
        assert_eq!(e.up(3, 1).to_string(), "F##"); // augmented 2nd above E
    }

    #[test]
    fn from_pc_spellings() {
        assert_eq!(NoteName::from_pc(10, Spelling::Flats).to_string(), "Bb");
        assert_eq!(NoteName::from_pc(10, Spelling::Sharps).to_string(), "A#");
        assert_eq!(NoteName::from_pc(4, Spelling::Flats).to_string(), "E");
    }

    #[test]
    fn frequencies() {
        assert!((Pitch(69).frequency() - 440.0).abs() < 1e-9);
        // Standard tuning: E2 A2 D3 G3 B3 E4
        let low_e = Pitch(40).frequency();
        assert!((low_e - 82.4069).abs() < 0.001);
        assert_eq!(Pitch(40).name(Spelling::Sharps), "E2");
        assert_eq!(Pitch(64).name(Spelling::Sharps), "E4");
        let (p, cents) = Pitch::nearest(110.0);
        assert_eq!(p, Pitch(45));
        assert!(cents.abs() < 1e-6);
        let (p, cents) = Pitch::nearest(Pitch(55).frequency() * 2f64.powf(10.0 / 1200.0));
        assert_eq!(p, Pitch(55));
        assert!((cents - 10.0).abs() < 1e-6);
    }

    #[test]
    fn simplified_names() {
        let e_sharp: NoteName = "E#".parse().unwrap();
        assert_eq!(e_sharp.simplified(Spelling::Sharps).to_string(), "F");
        let bbb: NoteName = "Bbb".parse().unwrap();
        assert_eq!(bbb.simplified(Spelling::Flats).to_string(), "A");
        let fs: NoteName = "F#".parse().unwrap();
        assert_eq!(fs.simplified(Spelling::Flats).to_string(), "F#");
    }
}
