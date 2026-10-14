//! Chord qualities, chord symbols ("F#m7", "Cadd9", "G/B"), and chord spelling.

use crate::interval::{Interval as I, Role};
use crate::note::{NoteName, ParseNoteError, PitchClass, Spelling, pc};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChordQuality {
    Major,
    Minor,
    Power,
    Sus2,
    Sus4,
    Dom7,
    Maj7,
    Min7,
    Six,
    MinorSix,
    Add9,
    MinorAdd9,
    Dom7Sus4,
    Dim,
    Aug,
    HalfDim7,
    Dim7,
    MinMaj7,
    Aug7,
    Dom9,
    Maj9,
    Min9,
    Dom11,
    Min11,
    Dom13,
}

/// One chord tone and whether a voicing may leave it out (usually the 5th).
#[derive(Clone, Copy, Debug)]
pub struct ChordTone {
    pub interval: I,
    pub required: bool,
}

const fn req(interval: I) -> ChordTone {
    ChordTone {
        interval,
        required: true,
    }
}
const fn opt(interval: I) -> ChordTone {
    ChordTone {
        interval,
        required: false,
    }
}

impl ChordQuality {
    pub const ALL: [ChordQuality; 25] = [
        ChordQuality::Major,
        ChordQuality::Minor,
        ChordQuality::Power,
        ChordQuality::Sus2,
        ChordQuality::Sus4,
        ChordQuality::Dom7,
        ChordQuality::Maj7,
        ChordQuality::Min7,
        ChordQuality::Six,
        ChordQuality::MinorSix,
        ChordQuality::Add9,
        ChordQuality::MinorAdd9,
        ChordQuality::Dom7Sus4,
        ChordQuality::Dim,
        ChordQuality::Aug,
        ChordQuality::HalfDim7,
        ChordQuality::Dim7,
        ChordQuality::MinMaj7,
        ChordQuality::Aug7,
        ChordQuality::Dom9,
        ChordQuality::Maj9,
        ChordQuality::Min9,
        ChordQuality::Dom11,
        ChordQuality::Min11,
        ChordQuality::Dom13,
    ];

    /// The symbol suffix FretStudy displays (C, Cm, C7, Cmaj7, Cm7♭5…).
    pub fn suffix(self) -> &'static str {
        use ChordQuality::*;
        match self {
            Major => "",
            Minor => "m",
            Power => "5",
            Sus2 => "sus2",
            Sus4 => "sus4",
            Dom7 => "7",
            Maj7 => "maj7",
            Min7 => "m7",
            Six => "6",
            MinorSix => "m6",
            Add9 => "add9",
            MinorAdd9 => "madd9",
            Dom7Sus4 => "7sus4",
            Dim => "dim",
            Aug => "aug",
            HalfDim7 => "m7b5",
            Dim7 => "dim7",
            MinMaj7 => "m(maj7)",
            Aug7 => "7#5",
            Dom9 => "9",
            Maj9 => "maj9",
            Min9 => "m9",
            Dom11 => "11",
            Min11 => "m11",
            Dom13 => "13",
        }
    }

    /// Plain-language name for lessons.
    pub fn name(self) -> &'static str {
        use ChordQuality::*;
        match self {
            Major => "major",
            Minor => "minor",
            Power => "power chord",
            Sus2 => "suspended 2nd",
            Sus4 => "suspended 4th",
            Dom7 => "dominant 7th",
            Maj7 => "major 7th",
            Min7 => "minor 7th",
            Six => "major 6th",
            MinorSix => "minor 6th",
            Add9 => "add 9",
            MinorAdd9 => "minor add 9",
            Dom7Sus4 => "7 sus 4",
            Dim => "diminished",
            Aug => "augmented",
            HalfDim7 => "half-diminished (minor 7 flat 5)",
            Dim7 => "diminished 7th",
            MinMaj7 => "minor-major 7th",
            Aug7 => "augmented 7th",
            Dom9 => "dominant 9th",
            Maj9 => "major 9th",
            Min9 => "minor 9th",
            Dom11 => "dominant 11th",
            Min11 => "minor 11th",
            Dom13 => "dominant 13th",
        }
    }

    pub fn tones(self) -> &'static [ChordTone] {
        use ChordQuality::*;
        match self {
            Major => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), req(I::P5)];
                T
            }
            Minor => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), req(I::P5)];
                T
            }
            Power => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::P5)];
                T
            }
            Sus2 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ2), req(I::P5)];
                T
            }
            Sus4 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::P4), req(I::P5)];
                T
            }
            Dom7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), opt(I::P5), req(I::MIN7)];
                T
            }
            Maj7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), opt(I::P5), req(I::MAJ7)];
                T
            }
            Min7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), opt(I::P5), req(I::MIN7)];
                T
            }
            Six => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), opt(I::P5), req(I::MAJ6)];
                T
            }
            MinorSix => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), opt(I::P5), req(I::MAJ6)];
                T
            }
            Add9 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), opt(I::P5), req(I::MAJ9)];
                T
            }
            MinorAdd9 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), opt(I::P5), req(I::MAJ9)];
                T
            }
            Dom7Sus4 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::P4), opt(I::P5), req(I::MIN7)];
                T
            }
            Dim => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), req(I::DIM5)];
                T
            }
            Aug => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), req(I::AUG5)];
                T
            }
            HalfDim7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), req(I::DIM5), req(I::MIN7)];
                T
            }
            Dim7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), req(I::DIM5), req(I::DIM7)];
                T
            }
            MinMaj7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MIN3), opt(I::P5), req(I::MAJ7)];
                T
            }
            Aug7 => {
                const T: &[ChordTone] = &[req(I::ROOT), req(I::MAJ3), req(I::AUG5), req(I::MIN7)];
                T
            }
            Dom9 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    req(I::MAJ3),
                    opt(I::P5),
                    req(I::MIN7),
                    req(I::MAJ9),
                ];
                T
            }
            Maj9 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    req(I::MAJ3),
                    opt(I::P5),
                    req(I::MAJ7),
                    req(I::MAJ9),
                ];
                T
            }
            Min9 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    req(I::MIN3),
                    opt(I::P5),
                    req(I::MIN7),
                    req(I::MAJ9),
                ];
                T
            }
            Dom11 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    opt(I::MAJ3),
                    opt(I::P5),
                    req(I::MIN7),
                    opt(I::MAJ9),
                    req(I::P11),
                ];
                T
            }
            Min11 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    req(I::MIN3),
                    opt(I::P5),
                    req(I::MIN7),
                    opt(I::MAJ9),
                    req(I::P11),
                ];
                T
            }
            Dom13 => {
                const T: &[ChordTone] = &[
                    req(I::ROOT),
                    req(I::MAJ3),
                    opt(I::P5),
                    req(I::MIN7),
                    opt(I::MAJ9),
                    req(I::MAJ13),
                ];
                T
            }
        }
    }

    /// Accepted spellings of each suffix, longest first when matching.
    fn aliases(self) -> &'static [&'static str] {
        use ChordQuality::*;
        match self {
            Major => &["", "maj", "M"],
            Minor => &["m", "min", "-"],
            Power => &["5"],
            Sus2 => &["sus2"],
            Sus4 => &["sus4", "sus"],
            Dom7 => &["7", "dom7"],
            Maj7 => &["maj7", "M7", "Δ7", "Δ", "ma7"],
            Min7 => &["m7", "min7", "-7"],
            Six => &["6"],
            MinorSix => &["m6", "min6"],
            Add9 => &["add9", "add2", "2"],
            MinorAdd9 => &["madd9", "m(add9)", "madd2"],
            Dom7Sus4 => &["7sus4", "7sus"],
            Dim => &["dim", "°", "o"],
            Aug => &["aug", "+"],
            HalfDim7 => &["m7b5", "m7♭5", "ø7", "ø", "min7b5", "-7b5"],
            Dim7 => &["dim7", "°7", "o7"],
            MinMaj7 => &["m(maj7)", "mMaj7", "mmaj7", "mM7", "m(M7)"],
            Aug7 => &["7#5", "7♯5", "aug7", "+7"],
            Dom9 => &["9"],
            Maj9 => &["maj9", "M9", "Δ9"],
            Min9 => &["m9", "min9", "-9"],
            Dom11 => &["11"],
            Min11 => &["m11", "min11", "-11"],
            Dom13 => &["13"],
        }
    }

    /// Finds the quality whose alias matches the whole suffix.
    pub fn from_suffix(suffix: &str) -> Option<ChordQuality> {
        ChordQuality::ALL
            .iter()
            .copied()
            .find(|q| q.aliases().contains(&suffix))
    }
}

/// A chord symbol: root + quality, with an optional different bass note (slash chord).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: NoteName,
    pub quality: ChordQuality,
    pub bass: Option<NoteName>,
}

impl Chord {
    pub fn new(root: NoteName, quality: ChordQuality) -> Self {
        Self {
            root,
            quality,
            bass: None,
        }
    }

    pub fn with_bass(mut self, bass: NoteName) -> Self {
        self.bass = Some(bass);
        self
    }

    /// The chord tones, spelled (Cm7 → C E♭ G B♭).
    pub fn notes(&self) -> Vec<NoteName> {
        self.quality
            .tones()
            .iter()
            .map(|t| t.interval.above(self.root))
            .collect()
    }

    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        let mut pcs: Vec<PitchClass> = self.notes().iter().map(|n| n.pc()).collect();
        pcs.sort_unstable();
        pcs.dedup();
        pcs
    }

    /// The interval of a pitch class above the root, if it's a chord tone.
    pub fn interval_of(&self, value: PitchClass) -> Option<I> {
        let distance = pc(value as i32 - self.root.pc() as i32);
        self.quality
            .tones()
            .iter()
            .map(|t| t.interval)
            .find(|i| i.semitones % 12 == distance)
    }

    pub fn role_of(&self, value: PitchClass) -> Option<Role> {
        self.interval_of(value).map(|i| i.role())
    }

    /// Checks whether a set of sounding pitch classes is a valid voicing of this chord:
    /// every required tone present, and nothing that isn't a chord tone.
    pub fn check_voicing(&self, sounding: &[PitchClass]) -> VoicingCheck {
        let allowed: Vec<PitchClass> = self.pitch_classes();
        let missing: Vec<NoteName> = self
            .quality
            .tones()
            .iter()
            .filter(|t| t.required)
            .map(|t| t.interval.above(self.root))
            .filter(|n| !sounding.contains(&n.pc()))
            .collect();
        let mut extra: Vec<PitchClass> = sounding
            .iter()
            .copied()
            .filter(|p| !allowed.contains(p))
            .collect();
        extra.sort_unstable();
        extra.dedup();
        VoicingCheck { missing, extra }
    }

    /// All chords whose voicing rules match the sounding pitch classes, best first.
    /// The preferred answer has the bass note as its root and the simplest quality.
    pub fn identify(sounding: &[PitchClass], bass: Option<PitchClass>) -> Vec<Chord> {
        let mut found = Vec::new();
        for root in 0..12u8 {
            for quality in ChordQuality::ALL {
                let chord = Chord::new(NoteName::from_pc(root, common_spelling(root)), quality);
                if chord.check_voicing(sounding).ok() {
                    found.push(chord);
                }
            }
        }
        found.sort_by_key(|c| {
            let root_in_bass = bass.map(|b| b == c.root.pc()).unwrap_or(true);
            let optional_missing = c
                .quality
                .tones()
                .iter()
                .filter(|t| !t.required && !sounding.contains(&t.interval.above(c.root).pc()))
                .count();
            (!root_in_bass, optional_missing, c.quality)
        });
        if let Some(b) = bass {
            for c in found.iter_mut() {
                if c.root.pc() != b {
                    c.bass = Some(NoteName::from_pc(b, common_spelling(b)));
                }
            }
        }
        found
    }

    /// The chord a number of half steps higher (used for capo and transposing).
    pub fn transpose(&self, semitones: i32, spelling: Spelling) -> Chord {
        Chord {
            root: self.root.transpose(semitones, spelling),
            quality: self.quality,
            bass: self.bass.map(|b| b.transpose(semitones, spelling)),
        }
    }
}

/// The everyday name guitarists use for each root (E♭ not D♯, F♯ not G♭).
pub fn common_spelling(root: PitchClass) -> Spelling {
    match root % 12 {
        1 | 3 | 8 | 10 => Spelling::Flats, // D♭, E♭, A♭, B♭
        _ => Spelling::Sharps,             // F♯
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoicingCheck {
    pub missing: Vec<NoteName>,
    pub extra: Vec<PitchClass>,
}

impl VoicingCheck {
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty()
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.root, self.quality.suffix())?;
        if let Some(b) = self.bass {
            write!(f, "/{b}")?;
        }
        Ok(())
    }
}

impl Chord {
    /// Symbol with ♯/♭ glyphs, for display.
    pub fn pretty(&self) -> String {
        let suffix = self.quality.suffix().replace('b', "♭").replace('#', "♯");
        let mut s = format!("{}{}", self.root.pretty(), suffix);
        if let Some(b) = self.bass {
            s.push('/');
            s.push_str(&b.pretty());
        }
        s
    }
}

impl FromStr for Chord {
    type Err = ParseNoteError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (root, rest) = NoteName::parse_prefix(s)?;
        // A slash introduces the bass note, except inside "6/9"-style suffixes (unsupported).
        let (suffix, bass) = match rest.rfind('/') {
            Some(i) => {
                let bass: NoteName = rest[i + 1..].parse()?;
                (&rest[..i], Some(bass))
            }
            None => (rest, None),
        };
        let quality =
            ChordQuality::from_suffix(suffix).ok_or_else(|| ParseNoteError(s.to_string()))?;
        Ok(Chord {
            root,
            quality,
            bass,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell(symbol: &str) -> String {
        let chord: Chord = symbol.parse().unwrap();
        chord
            .notes()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn spelling() {
        assert_eq!(spell("C"), "C E G");
        assert_eq!(spell("Am"), "A C E");
        assert_eq!(spell("D7"), "D F# A C");
        assert_eq!(spell("Cm7"), "C Eb G Bb"); // Learn & Master Session 20 example
        assert_eq!(spell("Dm7"), "D F A C");
        assert_eq!(spell("Bm7b5"), "B D F A");
        assert_eq!(spell("Bdim"), "B D F");
        assert_eq!(spell("Caug"), "C E G#");
        assert_eq!(spell("Csus4"), "C F G"); // Music Theory for the Music Industry
        assert_eq!(spell("Cdim7"), "C Eb Gb Bbb");
        assert_eq!(spell("C13"), "C E G Bb D A");
        assert_eq!(spell("F#m11"), "F# A C# E G# B");
        assert_eq!(spell("G5"), "G D");
    }

    #[test]
    fn parsing() {
        let c: Chord = "Bbmaj7".parse().unwrap();
        assert_eq!(c.to_string(), "Bbmaj7");
        let c: Chord = "G/B".parse().unwrap();
        assert_eq!(c.bass.unwrap().to_string(), "B");
        assert_eq!(c.pretty(), "G/B");
        let c: Chord = "F#m7b5".parse().unwrap();
        assert_eq!(c.quality, ChordQuality::HalfDim7);
        assert_eq!(c.pretty(), "F♯m7♭5");
        let c: Chord = "Dsus".parse().unwrap();
        assert_eq!(c.quality, ChordQuality::Sus4);
        assert!("Cxyz".parse::<Chord>().is_err());
    }

    #[test]
    fn voicing_check() {
        let g7: Chord = "G7".parse().unwrap();
        // G7 without the 5th is still a G7.
        assert!(g7.check_voicing(&[7, 11, 5]).ok());
        let c: Chord = "C".parse().unwrap();
        let bad = c.check_voicing(&[0, 4]);
        assert_eq!(
            bad.missing
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>(),
            ["G"]
        );
        let extra = c.check_voicing(&[0, 4, 7, 2]);
        assert_eq!(extra.extra, [2]);
    }

    #[test]
    fn identify_chords() {
        // C E G with C in the bass → C
        assert_eq!(Chord::identify(&[0, 4, 7], Some(0))[0].to_string(), "C");
        // A C E with A in the bass → Am
        assert_eq!(Chord::identify(&[9, 0, 4], Some(9))[0].to_string(), "Am");
        // C E G with E in the bass → C/E
        assert_eq!(Chord::identify(&[0, 4, 7], Some(4))[0].to_string(), "C/E");
        // D F# A C → D7
        assert_eq!(Chord::identify(&[2, 6, 9, 0], Some(2))[0].to_string(), "D7");
    }

    #[test]
    fn transposing() {
        let g: Chord = "G".parse().unwrap();
        assert_eq!(g.transpose(2, Spelling::Sharps).to_string(), "A");
        let c: Chord = "C".parse().unwrap();
        assert_eq!(c.transpose(3, Spelling::Flats).to_string(), "Eb");
    }
}
