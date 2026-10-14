//! The chord shapes taught in the FretStudy curriculum, with fingerings.
//! Every entry is checked by tests to spell the chord it claims to be.

use crate::chord::Chord;
use crate::shapes::ChordShape;

#[derive(Clone, Copy, Debug)]
pub struct ChordEntry {
    /// Unique id used by lessons, e.g. "G", "G-easy", "Em7-1".
    pub id: &'static str,
    /// Chord symbol the shape spells.
    pub symbol: &'static str,
    /// Display name in lessons.
    pub label: &'static str,
    /// Frets, thickest string first.
    pub frets: &'static str,
    /// Fingers, thickest string first (`-` = none).
    pub fingers: &'static str,
    /// Lesson that introduces it.
    pub lesson: &'static str,
}

impl ChordEntry {
    pub fn shape(&self) -> ChordShape {
        ChordShape::parse(self.frets, Some(self.fingers)).expect("library shapes are valid")
    }

    pub fn chord(&self) -> Chord {
        self.symbol.parse().expect("library symbols are valid")
    }
}

const fn e(
    id: &'static str,
    symbol: &'static str,
    label: &'static str,
    frets: &'static str,
    fingers: &'static str,
    lesson: &'static str,
) -> ChordEntry {
    ChordEntry {
        id,
        symbol,
        label,
        frets,
        fingers,
        lesson,
    }
}

pub const CHORDS: &[ChordEntry] = &[
    // Level 1 — First Strums
    e("Em", "Em", "E minor", "022000", "-23---", "L1.1"),
    e(
        "Em7-1",
        "Em7",
        "E minor 7 (one finger)",
        "020000",
        "-2----",
        "L1.1",
    ),
    e("G-easy", "G", "Easy G", "xx0003", "-----3", "L1.3"),
    e("C-easy", "C", "Easy C", "xx2010", "--2-1-", "L1.3"),
    e("D7", "D7", "D7", "xx0212", "---213", "L1.5"),
    // Level 2 — Open Chord Foundations
    e("G", "G", "G major", "320003", "21---3", "L2.2"),
    e(
        "G-alt",
        "G",
        "G major (3-2-4 fingering)",
        "320003",
        "32---4",
        "L2.2",
    ),
    e("C", "C", "C major", "x32010", "-32-1-", "L2.2"),
    e("D", "D", "D major", "xx0232", "---132", "L2.3"),
    e("A", "A", "A major", "x02220", "--123-", "L2.3"),
    e(
        "A-alt",
        "A",
        "A major (2-1-3 fingering)",
        "x02220",
        "--213-",
        "L2.3",
    ),
    e("E", "E", "E major", "022100", "-231--", "L2.4"),
    e("Am", "Am", "A minor", "x02210", "--231-", "L2.4"),
    e("Dm", "Dm", "D minor", "xx0231", "---231", "L2.5"),
    // Level 3 — Rhythm & Reading
    e("E7", "E7", "E7", "020100", "-2-1--", "L3.3"),
    e("A7", "A7", "A7", "x02020", "--2-3-", "L3.3"),
    e("G7", "G7", "G7", "320001", "32---1", "L3.3"),
    e("C7", "C7", "C7", "x32310", "-3241-", "L3.3"),
    e("B7", "B7", "B7", "x21202", "-213-4", "L3.3"),
    e("Asus2", "Asus2", "A sus 2", "x02200", "--12--", "L3.4"),
    e("Asus4", "Asus4", "A sus 4", "x02230", "--124-", "L3.4"),
    e("Dsus2", "Dsus2", "D sus 2", "xx0230", "---13-", "L3.4"),
    e("Dsus4", "Dsus4", "D sus 4", "xx0233", "---134", "L3.4"),
    e("Esus4", "Esus4", "E sus 4", "022200", "-234--", "L3.4"),
    e("Cadd9", "Cadd9", "C add 9", "x32030", "-32-4-", "L3.4"),
    e("Em7", "Em7", "E minor 7", "022030", "-12-4-", "L3.4"),
    e("Am7", "Am7", "A minor 7", "x02010", "--2-1-", "L3.4"),
    // Level 4 — Movable Shapes
    e("Fmaj7", "Fmaj7", "F major 7", "xx3210", "--321-", "L4.5"),
    e("F-mini", "F", "Mini F", "xx3211", "--3211", "L4.5"),
    e("F", "F", "F major (barre)", "133211", "134211", "L4.6"),
    e("Bm", "Bm", "B minor (barre)", "x24432", "-13421", "L4.8"),
    e("Bb", "Bb", "B♭ major (barre)", "x13331", "-12341", "L4.8"),
    e(
        "F#m",
        "F#m",
        "F♯ minor (barre)",
        "244222",
        "134111",
        "L6.13",
    ),
    // Level 5: dominant 7th barre chords for playing the blues in any key.
    e(
        "A7-barre",
        "A7",
        "A7 (E-shape barre, fret 5)",
        "575655",
        "131211",
        "L5.12",
    ),
    e(
        "D7-Abarre",
        "D7",
        "D7 (A-shape barre, fret 5)",
        "x57575",
        "-13141",
        "L5.12",
    ),
    e(
        "E7-Abarre",
        "E7",
        "E7 (A-shape barre, fret 7)",
        "x79797",
        "-13141",
        "L5.12",
    ),
    e(
        "G7-barre",
        "G7",
        "G7 (E-shape barre, fret 3)",
        "353433",
        "131211",
        "L5.12",
    ),
    e(
        "C7-Abarre",
        "C7",
        "C7 (A-shape barre, fret 3)",
        "x35353",
        "-13141",
        "L5.12",
    ),
    e(
        "C7-barre",
        "C7",
        "C7 (E-shape barre, fret 8)",
        "8 10 8 9 8 8",
        "131211",
        "L5.12",
    ),
    e(
        "F7-Abarre",
        "F7",
        "F7 (A-shape barre, fret 8)",
        "x 8 10 8 10 8",
        "-13141",
        "L5.12",
    ),
    e(
        "G7-Abarre",
        "G7",
        "G7 (A-shape barre, fret 10)",
        "x 10 12 10 12 10",
        "-13141",
        "L5.12",
    ),
    // Level 6: triads on the top three strings.
    e(
        "C-tri",
        "C",
        "C triad (root on the bottom)",
        "xxx553",
        "---341",
        "L6.6",
    ),
    e(
        "F-tri",
        "F",
        "F triad (5th on the bottom)",
        "xxx565",
        "---121",
        "L6.6",
    ),
    e(
        "G-tri",
        "G",
        "G triad (5th on the bottom)",
        "xxx787",
        "---121",
        "L6.6",
    ),
    e("E5", "E5", "E5 power chord", "022xxx", "-12---", "L4.3"),
    e("A5", "A5", "A5 power chord", "x022xx", "--12--", "L4.3"),
    e("D5", "D5", "D5 power chord", "xx023x", "---13-", "L4.3"),
    e("G5", "G5", "G5 power chord", "355xxx", "134---", "L4.3"),
    e("C5", "C5", "C5 power chord", "x355xx", "-134--", "L4.3"),
    e("B5", "B5", "B5 power chord", "x244xx", "-134--", "L4.3"),
    e("F5", "F5", "F5 power chord", "133xxx", "134---", "L4.3"),
    e(
        "A-barre",
        "A",
        "A major (E-shape barre, fret 5)",
        "577655",
        "134211",
        "L4.6",
    ),
    e("Fm", "Fm", "F minor (barre)", "133111", "134111", "L4.7"),
    e("F7", "F7", "F7 (barre)", "131211", "131211", "L4.7"),
    e(
        "Am-barre",
        "Am",
        "A minor (E-shape barre, fret 5)",
        "577555",
        "134111",
        "L4.7",
    ),
    e(
        "G-barre",
        "G",
        "G major (E-shape barre, fret 3)",
        "355433",
        "134211",
        "L4.7",
    ),
    e(
        "C-barre",
        "C",
        "C major (E-shape barre, fret 8)",
        "8 10 10 9 8 8",
        "134211",
        "L4.7",
    ),
    e(
        "C-Abarre",
        "C",
        "C major (A-shape barre, fret 3)",
        "x35553",
        "-12341",
        "L4.8",
    ),
    // Level 7: movable 7th chords (root on string 6 or 5) and color chords.
    e(
        "Fmaj7-6",
        "Fmaj7",
        "Fmaj7 (root on string 6)",
        "1x221x",
        "2-341-",
        "L7.2",
    ),
    e(
        "F7-6",
        "F7",
        "F7 (root on string 6)",
        "1x121x",
        "1-121-",
        "L7.2",
    ),
    e(
        "Fm7-6",
        "Fm7",
        "Fm7 (root on string 6)",
        "1x111x",
        "1-111-",
        "L7.2",
    ),
    e(
        "Cmaj7-5",
        "Cmaj7",
        "Cmaj7 (root on string 5)",
        "x35453",
        "-13241",
        "L7.2",
    ),
    e(
        "Cm7-5",
        "Cm7",
        "Cm7 (root on string 5)",
        "x35343",
        "-13121",
        "L7.2",
    ),
    e(
        "Cm7b5-5",
        "Cm7b5",
        "Cm7♭5 (root on string 5)",
        "x3434x",
        "-1324-",
        "L7.2",
    ),
    e(
        "Am7-6",
        "Am7",
        "Am7 (root on string 6, fret 5)",
        "5x555x",
        "1-111-",
        "L7.2",
    ),
    e(
        "Dm7-5",
        "Dm7",
        "Dm7 (root on string 5, fret 5)",
        "x57565",
        "-13121",
        "L7.2",
    ),
    e(
        "G7-6",
        "G7",
        "G7 (root on string 6, fret 3)",
        "3x343x",
        "1-121-",
        "L7.2",
    ),
    e(
        "Am7b5-6",
        "Am7b5",
        "Am7♭5 (root on string 6)",
        "5x554x",
        "2-341-",
        "L7.2",
    ),
    e("Cmaj9", "Cmaj9", "Cmaj9", "x32430", "-2143-", "L7.7"),
    e("C6", "C6", "C6", "x32210", "-3241-", "L7.7"),
    e("Am9", "Am9", "Am9", "x05557", "--1114", "L7.7"),
    e("C9", "C9", "C9", "x32333", "-21333", "L7.7"),
    e(
        "A13",
        "A13",
        "A13 (root on string 6)",
        "5x567x",
        "1-234-",
        "L7.7",
    ),
    // Style tracks: blues 9ths, 13ths, and diminished 7ths.
    e("E9", "E9", "E9", "020102", "-2-1-3", "B.3"),
    e("A9", "A9", "A9", "x02423", "--1312", "B.3"),
    e("B9", "B9", "B9", "x21222", "-21333", "B.3"),
    e(
        "D9",
        "D9",
        "D9 (root on string 5, fret 5)",
        "x54555",
        "-21333",
        "B.3",
    ),
    e("E13", "E13", "E13", "0x0120", "---12-", "B.3"),
    e(
        "Ddim7",
        "Ddim7",
        "D diminished 7",
        "xx0101",
        "---1-2",
        "B.7",
    ),
    e(
        "D#dim7",
        "D#dim7",
        "D♯ diminished 7",
        "xx1212",
        "--1324",
        "B.7",
    ),
    // Jazz shell voicings: root, 3rd, and 7th only.
    e(
        "G7-shell",
        "G7",
        "G7 shell (root, 7th, 3rd)",
        "3x34xx",
        "2-34--",
        "J.1",
    ),
    e(
        "Cmaj7-shell",
        "Cmaj7",
        "Cmaj7 shell",
        "x3x45x",
        "-1-23-",
        "J.1",
    ),
    e("Dm7-shell", "Dm7", "Dm7 shell", "x5x56x", "-1-23-", "J.1"),
    e("F7-shell", "F7", "F7 shell", "1x12xx", "1-23--", "J.7"),
    e("Bb7-shell", "Bb7", "B♭7 shell", "x1x13x", "-1-24-", "J.7"),
    e("C7-shell", "C7", "C7 shell", "x3x35x", "-1-24-", "J.7"),
    e("Gm7-shell", "Gm7", "Gm7 shell", "3x33xx", "2-34--", "J.7"),
    e("D7-shell", "D7", "D7 shell", "x5x57x", "-1-24-", "J.7"),
    e(
        "Bdim7",
        "Bdim7",
        "B diminished 7",
        "x2313x",
        "-2314-",
        "J.7",
    ),
];

/// A playable shape for any chord: the library shape closest to the nut that spells it,
/// or else a movable E-shape or A-shape barre. None if neither exists.
pub fn shape_for(chord: &Chord, tuning: &crate::Tuning) -> Option<ChordShape> {
    let want = chord.pitch_classes();
    CHORDS
        .iter()
        .filter(|e| {
            let c = e.chord();
            c.root.pc() == chord.root.pc() && c.pitch_classes() == want && c.bass.is_none()
        })
        .map(|e| e.shape())
        .min_by_key(|s| s.highest_fret().unwrap_or(0))
        .or_else(|| {
            [crate::BarreFamily::EShape, crate::BarreFamily::AShape]
                .iter()
                .filter_map(|f| f.shape(chord, tuning))
                .min_by_key(|s| s.lowest_fret().unwrap_or(0))
        })
}

pub fn find(id: &str) -> Option<&'static ChordEntry> {
    CHORDS.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fretboard::Tuning;

    #[test]
    fn every_library_shape_spells_its_chord() {
        let t = Tuning::standard();
        for entry in CHORDS {
            let shape = entry.shape();
            let check = shape.check(&entry.chord(), &t);
            assert!(
                check.ok(),
                "{} ({}) is wrong: {:?}",
                entry.id,
                entry.frets,
                check
            );
        }
    }

    #[test]
    fn every_fretted_string_has_a_finger() {
        for entry in CHORDS {
            let shape = entry.shape();
            for s in 0..6 {
                let fretted = matches!(shape.frets[s], Some(f) if f > 0);
                assert_eq!(
                    fretted,
                    shape.fingers[s].is_some(),
                    "{} string idx {s}",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = CHORDS.iter().map(|c| c.id).collect();
        ids.sort();
        let len = ids.len();
        ids.dedup();
        assert_eq!(len, ids.len());
    }

    #[test]
    fn major_minor_one_note_changes() {
        // L2.5: D→Dm, E→Em, A→Am each change exactly one string.
        for (maj, min) in [("D", "Dm"), ("E", "Em"), ("A", "Am")] {
            let a = find(maj).unwrap().shape();
            let b = find(min).unwrap().shape();
            let changed = (0..6).filter(|&s| a.frets[s] != b.frets[s]).count();
            assert_eq!(changed, 1, "{maj}→{min}");
        }
    }
}
