//! Chord shapes (what your fingers do) and generators for movable shapes:
//! barre chords, power chords, CAGED shapes, and scale boxes.

use crate::chord::{Chord, ChordQuality, VoicingCheck};
use crate::fretboard::{STRINGS, Tuning};
use crate::note::{Pitch, PitchClass};
use crate::scale::{Scale, ScaleKind};
use std::fmt;

/// One finger pressing several strings at the same fret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Barre {
    pub fret: u8,
    pub finger: u8,
    /// String indexes covered, thickest first (inclusive).
    pub from: usize,
    pub to: usize,
}

/// A chord shape: per string, `None` = don't play (x), `Some(0)` = open, `Some(n)` = fret n.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordShape {
    pub frets: [Option<u8>; STRINGS],
    /// Fretting finger per string (1 index … 4 pinky), when known.
    pub fingers: [Option<u8>; STRINGS],
    pub barres: Vec<Barre>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeParseError(pub String);

impl fmt::Display for ShapeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad chord shape: {:?}", self.0)
    }
}

impl std::error::Error for ShapeParseError {}

/// Splits "x32010" into per-string tokens, or "8 10 10 9 8 8" on whitespace.
fn tokens(s: &str) -> Result<Vec<String>, ShapeParseError> {
    let s = s.trim();
    let parts: Vec<String> = if s.contains(char::is_whitespace) {
        s.split_whitespace().map(str::to_string).collect()
    } else {
        s.chars().map(|c| c.to_string()).collect()
    };
    if parts.len() != STRINGS {
        return Err(ShapeParseError(s.to_string()));
    }
    Ok(parts)
}

impl ChordShape {
    /// Parses frets like `"x32010"` or `"8 10 10 9 8 8"`, and optional fingers like
    /// `"-32-1-"` (`-`, `x`, `0`, or `_` = no finger).
    pub fn parse(frets: &str, fingers: Option<&str>) -> Result<ChordShape, ShapeParseError> {
        let mut out = [None; STRINGS];
        for (i, t) in tokens(frets)?.iter().enumerate() {
            out[i] = match t.as_str() {
                "x" | "X" => None,
                n => Some(
                    n.parse::<u8>()
                        .map_err(|_| ShapeParseError(frets.to_string()))?,
                ),
            };
        }
        let mut finger_out = [None; STRINGS];
        if let Some(fs) = fingers {
            for (i, t) in tokens(fs)?.iter().enumerate() {
                finger_out[i] = match t.as_str() {
                    "-" | "x" | "X" | "0" | "_" => None,
                    "T" | "t" => Some(5),
                    n => Some(
                        n.parse::<u8>()
                            .map_err(|_| ShapeParseError(fs.to_string()))?,
                    ),
                };
            }
        }
        Ok(ChordShape::from_parts(out, finger_out))
    }

    /// Builds a shape and detects barres (one finger on 2+ strings at the same fret).
    pub fn from_parts(frets: [Option<u8>; STRINGS], fingers: [Option<u8>; STRINGS]) -> ChordShape {
        let mut barres = Vec::new();
        for finger in 1..=4u8 {
            let strings: Vec<usize> = (0..STRINGS)
                .filter(|&s| fingers[s] == Some(finger))
                .collect();
            if strings.len() < 2 {
                continue;
            }
            let fret = frets[strings[0]];
            if let Some(barre_fret) = fret
                && strings.iter().all(|&s| frets[s] == fret)
            {
                barres.push(Barre {
                    fret: barre_fret,
                    finger,
                    from: *strings.first().unwrap(),
                    to: *strings.last().unwrap(),
                });
            }
        }
        ChordShape {
            frets,
            fingers,
            barres,
        }
    }

    /// The sounding pitches (string index, pitch), thickest first.
    pub fn pitches(&self, tuning: &Tuning) -> Vec<(usize, Pitch)> {
        (0..STRINGS)
            .filter_map(|s| self.frets[s].map(|f| (s, tuning.pitch(s, f))))
            .collect()
    }

    pub fn pitch_classes(&self, tuning: &Tuning) -> Vec<PitchClass> {
        let mut pcs: Vec<PitchClass> = self.pitches(tuning).iter().map(|(_, p)| p.pc()).collect();
        pcs.sort_unstable();
        pcs.dedup();
        pcs
    }

    pub fn bass(&self, tuning: &Tuning) -> Option<Pitch> {
        self.pitches(tuning).first().map(|(_, p)| *p)
    }

    /// Does this shape correctly spell `chord`? (Slash chords also check the bass note.)
    pub fn check(&self, chord: &Chord, tuning: &Tuning) -> VoicingCheck {
        let mut result = chord.check_voicing(&self.pitch_classes(tuning));
        if let (Some(bass), Some(sounding)) = (chord.bass, self.bass(tuning))
            && bass.pc() != sounding.pc()
        {
            result.missing.push(bass);
        }
        result
    }

    /// Lowest fretted (non-open) fret, used for the "5fr" label.
    pub fn lowest_fret(&self) -> Option<u8> {
        self.frets
            .iter()
            .flatten()
            .copied()
            .filter(|f| *f > 0)
            .min()
    }

    pub fn highest_fret(&self) -> Option<u8> {
        self.frets.iter().flatten().copied().max()
    }

    /// True when every fretted note sits within the first few frets so the nut can be drawn.
    pub fn fits_at_nut(&self, window: u8) -> bool {
        self.highest_fret().unwrap_or(0) <= window
    }

    pub fn strings_played(&self) -> usize {
        self.frets.iter().filter(|f| f.is_some()).count()
    }

    /// Compact text form: "x32010", or space-separated if any fret ≥ 10.
    pub fn fret_string(&self) -> String {
        let wide = self.frets.iter().flatten().any(|f| *f >= 10);
        let parts: Vec<String> = self
            .frets
            .iter()
            .map(|f| f.map(|n| n.to_string()).unwrap_or_else(|| "x".into()))
            .collect();
        if wide {
            parts.join(" ")
        } else {
            parts.concat()
        }
    }

    /// Fingers that stay on the same string and fret when changing to `other` (anchor fingers).
    pub fn anchor_fingers(&self, other: &ChordShape) -> Vec<u8> {
        (0..STRINGS)
            .filter_map(|s| match (self.fingers[s], other.fingers[s]) {
                (Some(a), Some(b)) if a == b && self.frets[s] == other.frets[s] => Some(a),
                _ => None,
            })
            .collect()
    }

    /// Shifts every fretted note (used when a capo is on: shape frets are relative to the capo).
    pub fn shifted(&self, frets: u8) -> ChordShape {
        let moved = self.frets.map(|f| f.map(|n| n + frets));
        ChordShape::from_parts(moved, self.fingers)
    }
}

impl fmt::Display for ChordShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.fret_string())
    }
}

// ---------------------------------------------------------------------------
// Movable shapes
// ---------------------------------------------------------------------------

/// The five CAGED shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Caged {
    C,
    A,
    G,
    E,
    D,
}

impl Caged {
    pub const ORDER: [Caged; 5] = [Caged::C, Caged::A, Caged::G, Caged::E, Caged::D];

    /// Which string the shape's lowest root sits on (string index).
    pub fn root_string(self) -> usize {
        match self {
            Caged::C | Caged::A => 1,
            Caged::G | Caged::E => 0,
            Caged::D => 2,
        }
    }

    /// Fret offsets from the root fret `r` (thickest string first). `None` = not played.
    fn offsets(self) -> [Option<i32>; STRINGS] {
        match self {
            Caged::C => [None, Some(0), Some(-1), Some(-3), Some(-2), Some(-3)],
            Caged::A => [None, Some(0), Some(2), Some(2), Some(2), Some(0)],
            Caged::G => [Some(0), Some(-1), Some(-3), Some(-3), Some(-3), Some(0)],
            Caged::E => [Some(0), Some(2), Some(2), Some(1), Some(0), Some(0)],
            Caged::D => [None, None, Some(0), Some(2), Some(3), Some(2)],
        }
    }

    /// Smallest offset in the template (how far below the root the shape reaches).
    fn reach_below_root(self) -> i32 {
        self.offsets().iter().flatten().copied().min().unwrap_or(0)
    }

    /// The full major-chord form of this CAGED shape for a root. The lowest playable
    /// position is chosen (open position when possible).
    pub fn major_chord(self, root: PitchClass, tuning: &Tuning) -> ChordShape {
        let min_root_fret = (-self.reach_below_root()).max(0) as u8;
        let r = tuning.fret_for(self.root_string(), root, min_root_fret) as i32;
        let frets = self.offsets().map(|o| o.map(|d| (r + d) as u8));
        ChordShape::from_parts(frets, [None; STRINGS])
    }
}

/// Relative frets and fingers for a movable shape, thickest string first.
type ShapeTemplate = ([Option<u8>; STRINGS], [Option<u8>; STRINGS]);

/// Barre-chord families built from open E and A shapes (Levels 4–5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarreFamily {
    /// Root on string 6.
    EShape,
    /// Root on string 5.
    AShape,
}

impl BarreFamily {
    /// Template (frets relative to the barre, fingers) for supported qualities.
    fn template(self, quality: ChordQuality) -> Option<ShapeTemplate> {
        use ChordQuality::*;
        let (f, g): ([Option<u8>; 6], [Option<u8>; 6]) = match (self, quality) {
            (BarreFamily::EShape, Major) => (
                [Some(0), Some(2), Some(2), Some(1), Some(0), Some(0)],
                [Some(1), Some(3), Some(4), Some(2), Some(1), Some(1)],
            ),
            (BarreFamily::EShape, Minor) => (
                [Some(0), Some(2), Some(2), Some(0), Some(0), Some(0)],
                [Some(1), Some(3), Some(4), Some(1), Some(1), Some(1)],
            ),
            (BarreFamily::EShape, Dom7) => (
                [Some(0), Some(2), Some(0), Some(1), Some(0), Some(0)],
                [Some(1), Some(3), Some(1), Some(2), Some(1), Some(1)],
            ),
            (BarreFamily::EShape, Min7) => (
                [Some(0), Some(2), Some(0), Some(0), Some(0), Some(0)],
                [Some(1), Some(3), Some(1), Some(1), Some(1), Some(1)],
            ),
            (BarreFamily::EShape, Power) => (
                [Some(0), Some(2), Some(2), None, None, None],
                [Some(1), Some(3), Some(4), None, None, None],
            ),
            (BarreFamily::AShape, Major) => (
                [None, Some(0), Some(2), Some(2), Some(2), Some(0)],
                [None, Some(1), Some(2), Some(3), Some(4), Some(1)],
            ),
            (BarreFamily::AShape, Minor) => (
                [None, Some(0), Some(2), Some(2), Some(1), Some(0)],
                [None, Some(1), Some(3), Some(4), Some(2), Some(1)],
            ),
            (BarreFamily::AShape, Dom7) => (
                [None, Some(0), Some(2), Some(0), Some(2), Some(0)],
                [None, Some(1), Some(3), Some(1), Some(4), Some(1)],
            ),
            (BarreFamily::AShape, Min7) => (
                [None, Some(0), Some(2), Some(0), Some(1), Some(0)],
                [None, Some(1), Some(3), Some(1), Some(2), Some(1)],
            ),
            (BarreFamily::EShape, Maj7) => (
                [Some(0), None, Some(1), Some(1), Some(0), None],
                [Some(2), None, Some(3), Some(4), Some(1), None],
            ),
            (BarreFamily::AShape, Maj7) => (
                [None, Some(0), Some(2), Some(1), Some(2), Some(0)],
                [None, Some(1), Some(3), Some(2), Some(4), Some(1)],
            ),
            (BarreFamily::AShape, Dim) => (
                [None, Some(0), Some(1), Some(2), Some(1), None],
                [None, Some(1), Some(2), Some(4), Some(3), None],
            ),
            (BarreFamily::AShape, HalfDim7) => (
                [None, Some(0), Some(1), Some(0), Some(1), None],
                [None, Some(1), Some(3), Some(2), Some(4), None],
            ),
            (BarreFamily::AShape, Power) => (
                [None, Some(0), Some(2), Some(2), None, None],
                [None, Some(1), Some(3), Some(4), None, None],
            ),
            _ => return None,
        };
        Some((f, g))
    }

    pub fn root_string(self) -> usize {
        match self {
            BarreFamily::EShape => 0,
            BarreFamily::AShape => 1,
        }
    }

    /// The movable shape for a chord, with the barre at fret 1–12.
    pub fn shape(self, chord: &Chord, tuning: &Tuning) -> Option<ChordShape> {
        let (frets, fingers) = self.template(chord.quality)?;
        let r = tuning.fret_for(self.root_string(), chord.root.pc(), 1);
        let frets = frets.map(|f| f.map(|d| d + r));
        let mut shape = ChordShape::from_parts(frets, fingers);
        if chord.quality == ChordQuality::Power {
            shape.barres.clear(); // power chords use separate fingers, not a barre
        }
        Some(shape)
    }
}

// ---------------------------------------------------------------------------
// Scale shapes
// ---------------------------------------------------------------------------

/// A note position on the neck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NeckNote {
    pub string: usize,
    pub fret: u8,
}

/// Builds a scale pattern by walking up the scale `per_string` notes at a time,
/// starting from scale degree `start_degree` (0-based) on the thickest string.
/// Two per string gives the 5 pentatonic boxes; three per string gives 3-notes-per-string
/// major-scale patterns. The lowest position at or above `min_fret` is returned.
pub fn scale_pattern(
    scale: &Scale,
    start_degree: usize,
    per_string: usize,
    min_fret: u8,
    tuning: &Tuning,
) -> Vec<NeckNote> {
    let pcs = scale.pitch_classes();
    let start_pc = pcs[start_degree % pcs.len()];
    let mut start_fret = tuning.fret_for(0, start_pc, min_fret);
    loop {
        if let Some(notes) = try_pattern(&pcs, start_degree, per_string, start_fret, tuning) {
            return notes;
        }
        start_fret += 12;
    }
}

fn try_pattern(
    pcs: &[PitchClass],
    start_degree: usize,
    per_string: usize,
    start_fret: u8,
    tuning: &Tuning,
) -> Option<Vec<NeckNote>> {
    let mut notes = Vec::new();
    let mut pitch = tuning.pitch(0, start_fret).0;
    let mut degree = start_degree;
    for string in 0..STRINGS {
        for _ in 0..per_string {
            let fret = pitch - tuning.open[string].0;
            if !(0..=24).contains(&fret) {
                return None;
            }
            notes.push(NeckNote {
                string,
                fret: fret as u8,
            });
            // advance to the next scale note
            let next = (degree + 1) % pcs.len();
            let step = (pcs[next] as i32 - pcs[degree] as i32).rem_euclid(12);
            pitch += if step == 0 { 12 } else { step };
            degree = next;
        }
    }
    Some(notes)
}

/// Pentatonic box `shape` (1–5) for a minor-pentatonic root, as taught in Levels 5–6.
/// Box 1 starts on the root on string 6. For a major pentatonic, pass its relative minor.
pub fn pentatonic_box(
    minor_root: PitchClass,
    shape: usize,
    min_fret: u8,
    tuning: &Tuning,
) -> Vec<NeckNote> {
    assert!(
        (1..=5).contains(&shape),
        "pentatonic shapes are numbered 1–5"
    );
    let root =
        crate::note::NoteName::from_pc(minor_root, crate::chord::common_spelling(minor_root));
    let scale = Scale::new(root, ScaleKind::MinorPentatonic);
    scale_pattern(&scale, shape - 1, 2, min_fret, tuning)
}

/// What chord a shape *sounds* like with a capo: `capo` half steps above the shape's name.
pub fn capo_sounding(shape_chord: &Chord, capo: u8) -> Chord {
    let target = (shape_chord.root.pc() as i32 + capo as i32).rem_euclid(12) as PitchClass;
    shape_chord.transpose(capo as i32, crate::chord::common_spelling(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(s: &str) -> Chord {
        s.parse().unwrap()
    }

    fn frets_of(notes: &[NeckNote], string: usize) -> Vec<u8> {
        notes
            .iter()
            .filter(|n| n.string == string)
            .map(|n| n.fret)
            .collect()
    }

    #[test]
    fn parse_and_check() {
        let t = Tuning::standard();
        let c = ChordShape::parse("x32010", Some("-32-1-")).unwrap();
        assert!(c.check(&chord("C"), &t).ok());
        assert_eq!(c.fingers[1], Some(3));
        let f = ChordShape::parse("133211", Some("134211")).unwrap();
        assert_eq!(
            f.barres,
            vec![Barre {
                fret: 1,
                finger: 1,
                from: 0,
                to: 5
            }]
        );
        assert!(f.check(&chord("F"), &t).ok());
        let wide = ChordShape::parse("8 10 10 9 8 8", None).unwrap();
        assert!(wide.check(&chord("C"), &t).ok());
        assert_eq!(wide.fret_string(), "8 10 10 9 8 8");
        assert_eq!(wide.lowest_fret(), Some(8));
        // A wrong shape is caught.
        let bad = ChordShape::parse("x32000", None).unwrap();
        assert!(!bad.check(&chord("C"), &t).ok());
    }

    #[test]
    fn anchors() {
        // L2.7: C ↔ Am keeps fingers 1 and 2
        let c = ChordShape::parse("x32010", Some("-32-1-")).unwrap();
        let am = ChordShape::parse("x02210", Some("--231-")).unwrap();
        let mut a = c.anchor_fingers(&am);
        a.sort();
        assert_eq!(a, vec![1, 2]);
    }

    #[test]
    fn caged_c_major() {
        // L6.5: C major in all five shapes
        let t = Tuning::standard();
        let expected = [
            (Caged::C, "x32010"),
            (Caged::A, "x35553"),
            (Caged::G, "875558"),
            (Caged::E, "8 10 10 9 8 8"),
            (Caged::D, "x x 10 12 13 12"),
        ];
        for (shape, frets) in expected {
            let s = shape.major_chord(0, &t);
            assert_eq!(s.fret_string(), frets, "{shape:?}");
            assert!(s.check(&chord("C"), &t).ok(), "{shape:?}");
        }
        // Every shape spells a correct major chord in every key.
        for root in 0..12 {
            let name = crate::note::NoteName::from_pc(root, crate::chord::common_spelling(root));
            for shape in Caged::ORDER {
                let s = shape.major_chord(root, &t);
                assert!(
                    s.check(&Chord::new(name, ChordQuality::Major), &t).ok(),
                    "{shape:?} {name}"
                );
            }
        }
    }

    #[test]
    fn barre_families() {
        let t = Tuning::standard();
        let e = BarreFamily::EShape;
        let a = BarreFamily::AShape;
        assert_eq!(e.shape(&chord("F"), &t).unwrap().fret_string(), "133211");
        assert_eq!(e.shape(&chord("Fm"), &t).unwrap().fret_string(), "133111");
        assert_eq!(e.shape(&chord("F7"), &t).unwrap().fret_string(), "131211");
        assert_eq!(e.shape(&chord("Fm7"), &t).unwrap().fret_string(), "131111");
        assert_eq!(e.shape(&chord("A"), &t).unwrap().fret_string(), "577655");
        assert_eq!(a.shape(&chord("B"), &t).unwrap().fret_string(), "x24442");
        assert_eq!(a.shape(&chord("Bm"), &t).unwrap().fret_string(), "x24432");
        assert_eq!(a.shape(&chord("B7"), &t).unwrap().fret_string(), "x24242");
        assert_eq!(a.shape(&chord("Bm7"), &t).unwrap().fret_string(), "x24232");
        assert_eq!(e.shape(&chord("G5"), &t).unwrap().fret_string(), "355xxx");
        assert_eq!(a.shape(&chord("C5"), &t).unwrap().fret_string(), "x355xx");
        // All 12 roots × all supported qualities spell correctly.
        for root in 0..12u8 {
            let name = crate::note::NoteName::from_pc(root, crate::chord::common_spelling(root));
            for q in [
                ChordQuality::Major,
                ChordQuality::Minor,
                ChordQuality::Dom7,
                ChordQuality::Min7,
                ChordQuality::Power,
            ] {
                for fam in [e, a] {
                    let c = Chord::new(name, q);
                    let s = fam.shape(&c, &t).unwrap();
                    assert!(s.check(&c, &t).ok(), "{fam:?} {c}");
                }
            }
        }
    }

    #[test]
    fn pentatonic_boxes_match_curriculum() {
        let t = Tuning::standard();
        // L5.1 Box 1, A minor, fret 5
        let b1 = pentatonic_box(9, 1, 1, &t);
        let expect1: [&[u8]; 6] = [&[5, 8], &[5, 7], &[5, 7], &[5, 7], &[5, 8], &[5, 8]];
        for s in 0..6 {
            assert_eq!(frets_of(&b1, s), expect1[s], "box1 string idx {s}");
        }
        // L6.3 shape 2 and 3, L6.4 shapes 4 and 5 (A minor)
        let b2 = pentatonic_box(9, 2, 5, &t);
        let e2: [&[u8]; 6] = [&[8, 10], &[7, 10], &[7, 10], &[7, 9], &[8, 10], &[8, 10]];
        let b3 = pentatonic_box(9, 3, 5, &t);
        let e3: [&[u8]; 6] = [
            &[10, 12],
            &[10, 12],
            &[10, 12],
            &[9, 12],
            &[10, 13],
            &[10, 12],
        ];
        let b4 = pentatonic_box(9, 4, 10, &t);
        let e4: [&[u8]; 6] = [
            &[12, 15],
            &[12, 15],
            &[12, 14],
            &[12, 14],
            &[13, 15],
            &[12, 15],
        ];
        let b5 = pentatonic_box(9, 5, 1, &t);
        let e5: [&[u8]; 6] = [&[3, 5], &[3, 5], &[2, 5], &[2, 5], &[3, 5], &[3, 5]];
        for s in 0..6 {
            assert_eq!(frets_of(&b2, s), e2[s], "box2 {s}");
            assert_eq!(frets_of(&b3, s), e3[s], "box3 {s}");
            assert_eq!(frets_of(&b4, s), e4[s], "box4 {s}");
            assert_eq!(frets_of(&b5, s), e5[s], "box5 {s}");
        }
        // Every note in every box, in every key, belongs to the scale.
        for root in 0..12 {
            for shape in 1..=5 {
                for n in pentatonic_box(root, shape, 0, &t) {
                    let p = t.pitch(n.string, n.fret).pc();
                    assert!([0, 3, 5, 7, 10].contains(&((p as i32 - root as i32).rem_euclid(12))));
                }
            }
        }
    }

    #[test]
    fn capo_math() {
        // L4.9 examples
        assert_eq!(capo_sounding(&chord("G"), 2).to_string(), "A");
        assert_eq!(capo_sounding(&chord("C"), 3).to_string(), "Eb");
        assert_eq!(capo_sounding(&chord("D"), 5).to_string(), "G");
    }
}
