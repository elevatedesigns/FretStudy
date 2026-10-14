//! Every playable way to play a chord: its inversions on each set of strings, and full
//! shapes up the neck (the five CAGED positions and their relatives).
//!
//! Voicings are found by searching the fretboard rather than copied from a table, so they
//! exist for any root and chord type, and every one is checked against the chord's notes.
//! A shape is kept only if one hand can play it: four frets or less, four fingers or less
//! (finger 1 may barre), and nothing ringing that isn't in the chord.

use crate::chord::Chord;
use crate::fretboard::{STRINGS, Tuning};
use crate::note::PitchClass;
use crate::shapes::ChordShape;
use std::collections::HashSet;

/// What a voicing is good for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Three or four notes on neighboring strings, each chord tone once.
    Inversion,
    /// A full chord with the root in the bass.
    Neck,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Voicing {
    pub shape: ChordShape,
    pub group: Group,
    /// "Root position", "1st inversion", … (which chord tone is in the bass).
    pub inversion: usize,
    /// e.g. "Strings 3-2-1 · 1st inversion" or "Full chord · fret 8".
    pub label: String,
}

impl Voicing {
    /// The lowest fret pressed (0 for all-open shapes).
    pub fn position(&self) -> u8 {
        self.shape.lowest_fret().unwrap_or(0)
    }
}

pub fn inversion_name(k: usize) -> &'static str {
    [
        "Root position",
        "1st inversion",
        "2nd inversion",
        "3rd inversion",
        "4th inversion",
    ]
    .get(k)
    .copied()
    .unwrap_or("Inversion")
}

/// Strings as players name them ("3-2-1"), from string indexes (0 = thickest).
fn set_name(set: &[usize]) -> String {
    set.iter()
        .map(|s| crate::fretboard::string_number(*s).to_string())
        .collect::<Vec<_>>()
        .join("-")
}

/// The chord's tones from the root up, as pitch classes (root first).
fn chord_tones(chord: &Chord) -> Vec<(PitchClass, bool)> {
    chord
        .quality
        .tones()
        .iter()
        .map(|t| {
            (
                ((chord.root.pc() as u32 + t.interval.semitones as u32) % 12) as PitchClass,
                t.required,
            )
        })
        .collect()
}

/// Which finger goes where, or None if one hand can't play it. Finger 1 barres the lowest
/// fret when more than four notes need fretting (and nothing between is open or muted).
pub fn assign_fingers(frets: &[Option<u8>; STRINGS]) -> Option<[Option<u8>; STRINGS]> {
    let notes: Vec<(usize, u8)> = (0..STRINGS)
        .filter_map(|s| frets[s].filter(|f| *f > 0).map(|f| (s, f)))
        .collect();
    let mut fingers = [None; STRINGS];
    if notes.is_empty() {
        return Some(fingers);
    }
    let lo = notes.iter().map(|n| n.1).min()?;
    let hi = notes.iter().map(|n| n.1).max()?;
    if hi - lo > 3 {
        return None;
    }
    // The usual ways first: one finger per note, then finger 1 barring the lowest fret.
    // Only then small barres with other fingers (like the ring finger in an A shape).
    let at_lo: Vec<usize> = notes.iter().filter(|n| n.1 == lo).map(|n| n.0).collect();
    let can_barre = at_lo.len() >= 2 && {
        let (from, to) = (at_lo[0], at_lo[at_lo.len() - 1]);
        (from..=to).all(|s| frets[s].is_some_and(|f| f >= lo))
    };
    let rest: Vec<(usize, u8)> = notes.iter().copied().filter(|n| n.1 > lo).collect();
    let with_barre = |mini: bool| {
        let mut out = place(&rest, lo, 2, frets, mini)?;
        for s in &at_lo {
            out[*s] = Some(1);
        }
        Some(out)
    };
    if let Some(f) = place(&notes, lo, 1, frets, false) {
        return Some(f);
    }
    if can_barre && let Some(f) = with_barre(false) {
        return Some(f);
    }
    if can_barre && let Some(f) = with_barre(true) {
        return Some(f);
    }
    let _ = &mut fingers;
    place(&notes, lo, 1, frets, true)
}

/// Gives each note a finger, starting at `first`. Fingers go up with the fret; notes on the
/// same fret may share one finger as a small barre when there aren't enough fingers.
fn place(
    notes: &[(usize, u8)],
    lo: u8,
    first: u8,
    frets: &[Option<u8>; STRINGS],
    mini: bool,
) -> Option<[Option<u8>; STRINGS]> {
    let mut out = [None; STRINGS];
    let mut by_fret: Vec<(u8, Vec<usize>)> = Vec::new();
    for &(s, f) in notes {
        match by_fret.iter_mut().find(|(ff, _)| *ff == f) {
            Some((_, v)) => v.push(s),
            None => by_fret.push((f, vec![s])),
        }
    }
    by_fret.sort_by_key(|(f, _)| *f);
    // How many fingers each fret group needs: one per note, or one if it can be a mini-barre
    // (neighboring strings, nothing lower in between) and fingers are short.
    let mut need: Vec<u8> = by_fret.iter().map(|(_, v)| v.len() as u8).collect();
    let available = 5 - first;
    let mut total: u8 = need.iter().sum();
    for (i, (f, strings)) in by_fret.iter().enumerate().rev() {
        if total <= available || !mini {
            break;
        }
        let (a, b) = (*strings.iter().min()?, *strings.iter().max()?);
        let solid = strings.len() > 1 && (a..=b).all(|s| frets[s].is_some_and(|x| x >= *f));
        if solid {
            total -= need[i] - 1;
            need[i] = 1;
        }
    }
    if total > available {
        return None;
    }
    let mut last = first - 1;
    let mut left = total;
    for (i, (f, strings)) in by_fret.iter().enumerate() {
        left -= need[i];
        // Prefer one finger per fret, but leave enough fingers for the notes still to come.
        let start = (last + 1).max(
            (first + f - lo)
                .saturating_sub(if first == 2 { 1 } else { 0 })
                .min(4 - left - (need[i] - 1)),
        );
        let start = start.max(last + 1);
        if start + need[i] - 1 > 4 {
            return None;
        }
        let mut sorted = strings.clone();
        sorted.sort_unstable();
        for (k, s) in sorted.iter().enumerate() {
            out[*s] = Some(if need[i] == 1 { start } else { start + k as u8 });
        }
        last = start + need[i] - 1;
    }
    Some(out)
}

/// All shapes that play exactly the strings in `set` (others muted) within a four-fret
/// window, using only chord tones and including every required tone.
fn search(chord: &Chord, tuning: &Tuning, set: &[usize], each_once: bool) -> Vec<ChordShape> {
    let tones = chord_tones(chord);
    let pcs: Vec<PitchClass> = tones.iter().map(|t| t.0).collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for lo in 0..=14u8 {
        // Fret options for each string in this window (open strings only near the nut).
        let options: Vec<Vec<u8>> = set
            .iter()
            .map(|&s| {
                let mut v: Vec<u8> = (lo.max(1)..=lo + 3)
                    .filter(|f| pcs.contains(&tuning.pitch(s, *f).pc()))
                    .collect();
                if lo <= 1 && pcs.contains(&tuning.pitch(s, 0).pc()) {
                    v.insert(0, 0);
                }
                v
            })
            .collect();
        if options.iter().any(Vec::is_empty) {
            continue;
        }
        let mut idx = vec![0usize; set.len()];
        'combos: loop {
            let mut frets = [None; STRINGS];
            for (k, &s) in set.iter().enumerate() {
                frets[s] = Some(options[k][idx[k]]);
            }
            if seen.insert(frets) {
                let sounding: Vec<PitchClass> = set
                    .iter()
                    .map(|&s| tuning.pitch(s, frets[s].unwrap_or(0)).pc())
                    .collect();
                let has_all = tones
                    .iter()
                    .filter(|t| t.1)
                    .all(|t| sounding.contains(&t.0));
                let once = !each_once || {
                    let mut u = sounding.clone();
                    u.sort_unstable();
                    u.dedup();
                    u.len() == sounding.len()
                };
                if has_all
                    && once
                    && let Some(fingers) = assign_fingers(&frets)
                {
                    out.push(ChordShape::from_parts(frets, fingers));
                }
            }
            // Next combination (like an odometer); done when every digit wraps.
            let mut k = 0;
            loop {
                if k == idx.len() {
                    break 'combos;
                }
                idx[k] += 1;
                if idx[k] < options[k].len() {
                    break;
                }
                idx[k] = 0;
                k += 1;
            }
        }
    }
    out
}

/// Which chord tone is lowest: 0 = root position, 1 = 1st inversion, and so on.
fn bass_tone(chord: &Chord, shape: &ChordShape, tuning: &Tuning) -> usize {
    let tones = chord_tones(chord);
    shape
        .bass(tuning)
        .and_then(|p| tones.iter().position(|t| t.0 == p.pc()))
        .unwrap_or(0)
}

/// Inversions on each group of neighboring strings: triads on four sets of three strings,
/// 7th chords on three sets of four. One voicing per set and inversion, lowest on the neck.
pub fn inversions(chord: &Chord, tuning: &Tuning) -> Vec<Voicing> {
    let n = chord_tones(chord).len();
    let sets: Vec<Vec<usize>> = match n {
        3 => vec![vec![3, 4, 5], vec![2, 3, 4], vec![1, 2, 3], vec![0, 1, 2]],
        4 => vec![
            vec![2, 3, 4, 5],
            vec![1, 2, 3, 4],
            vec![0, 1, 2, 3],
            vec![0, 2, 3, 4],
        ],
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for set in sets {
        let mut found: Vec<(usize, ChordShape)> = search(chord, tuning, &set, true)
            .into_iter()
            .map(|s| (bass_tone(chord, &s, tuning), s))
            .collect();
        found.sort_by_key(|(inv, s)| {
            (
                *inv,
                s.lowest_fret().unwrap_or(0),
                s.highest_fret().unwrap_or(0),
            )
        });
        for inv in 0..n {
            if let Some((_, shape)) = found.iter().find(|(i, _)| *i == inv) {
                let skip = if set == [0, 2, 3, 4] {
                    " (skip string 5)"
                } else {
                    ""
                };
                out.push(Voicing {
                    shape: shape.clone(),
                    group: Group::Inversion,
                    inversion: inv,
                    label: format!("Strings {}{skip} · {}", set_name(&set), inversion_name(inv)),
                });
            }
        }
    }
    out
}

/// Full chords (five or six strings, or four on top for small shapes) with the root in the
/// bass, from the nut up to fret 15, like the five CAGED positions.
pub fn neck_shapes(chord: &Chord, tuning: &Tuning) -> Vec<Voicing> {
    let root = chord.root.pc();
    // Full shapes, plus the partial shapes players use where a full one won't fit the hand
    // (like the G-shape position).
    let sets: [&[usize]; 6] = [
        &[0, 1, 2, 3, 4, 5],
        &[1, 2, 3, 4, 5],
        &[0, 1, 2, 3, 4],
        &[2, 3, 4, 5],
        &[1, 2, 3, 4],
        &[0, 1, 2, 3],
    ];
    let mut all: Vec<ChordShape> = Vec::new();
    for set in sets {
        for shape in search(chord, tuning, set, false) {
            if shape.bass(tuning).is_some_and(|p| p.pc() == root) {
                all.push(shape);
            }
        }
    }
    // Prefer fuller shapes, then lower ones; keep one shape per area of the neck and root string.
    all.sort_by_key(|s| {
        (
            s.lowest_fret().unwrap_or(0),
            std::cmp::Reverse(s.strings_played()),
        )
    });
    let mut out: Vec<Voicing> = Vec::new();
    let mut taken: Vec<(u8, usize)> = Vec::new();
    for shape in all {
        let pos = shape.lowest_fret().unwrap_or(0);
        let root_string = (0..STRINGS)
            .find(|s| shape.frets[*s].is_some())
            .unwrap_or(0);
        if taken.iter().any(|(p, r)| *r == root_string && *p == pos) {
            continue;
        }
        taken.push((pos, root_string));
        out.push(Voicing {
            label: if shape.frets.contains(&Some(0)) {
                format!(
                    "Open position · root on string {}",
                    crate::fretboard::string_number(root_string)
                )
            } else {
                format!(
                    "Fret {pos} · root on string {}",
                    crate::fretboard::string_number(root_string)
                )
            },
            shape,
            group: Group::Neck,
            inversion: 0,
        });
    }
    out.sort_by_key(|v| v.position());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::ChordQuality;
    use crate::note::{NoteName, Spelling};

    fn chord(root: u8, q: ChordQuality) -> Chord {
        Chord::new(NoteName::from_pc(root, Spelling::Sharps), q)
    }

    #[test]
    fn triads_have_every_inversion_on_every_string_set() {
        let t = Tuning::standard();
        for root in 0..12 {
            for q in [
                ChordQuality::Major,
                ChordQuality::Minor,
                ChordQuality::Dim,
                ChordQuality::Aug,
                ChordQuality::Sus2,
                ChordQuality::Sus4,
            ] {
                let c = chord(root, q);
                let v = inversions(&c, &t);
                // A couple of diminished/augmented inversions need a five-fret stretch, so
                // they're left out on purpose.
                let want = if matches!(q, ChordQuality::Major | ChordQuality::Minor) {
                    12
                } else {
                    10
                };
                assert!(v.len() >= want, "{}: {} inversions", c.pretty(), v.len());
                for x in &v {
                    assert!(
                        x.shape.check(&c, &t).ok(),
                        "{} {} doesn't spell it",
                        c.pretty(),
                        x.shape
                    );
                    assert_eq!(x.shape.strings_played(), 3);
                    assert!(x.shape.highest_fret().unwrap_or(0) <= 17);
                }
            }
        }
    }

    #[test]
    fn known_triad_inversions_on_the_top_strings() {
        let t = Tuning::standard();
        let c = chord(0, ChordQuality::Major); // C
        let v = inversions(&c, &t);
        let frets = |label: &str| {
            v.iter()
                .find(|x| x.label == label)
                .map(|x| x.shape.fret_string())
                .unwrap_or_default()
        };
        // The three C triads on strings 3-2-1, as taught in L6.6.
        assert_eq!(frets("Strings 3-2-1 · Root position"), "xxx553");
        assert_eq!(frets("Strings 3-2-1 · 2nd inversion"), "xxx010");
        assert_eq!(frets("Strings 3-2-1 · 1st inversion"), "xxx988");
    }

    #[test]
    fn seventh_chords_have_four_inversions_on_the_top_four_strings() {
        let t = Tuning::standard();
        for root in 0..12 {
            for q in [
                ChordQuality::Maj7,
                ChordQuality::Dom7,
                ChordQuality::Min7,
                ChordQuality::HalfDim7,
            ] {
                let c = chord(root, q);
                let v = inversions(&c, &t);
                let top: Vec<_> = v
                    .iter()
                    .filter(|x| x.label.starts_with("Strings 4-3-2-1"))
                    .collect();
                assert_eq!(top.len(), 4, "{}", c.pretty());
                for x in &v {
                    assert!(x.shape.check(&c, &t).ok(), "{} {}", c.pretty(), x.shape);
                }
            }
        }
    }

    #[test]
    fn neck_shapes_cover_the_neck_for_major_and_minor() {
        let t = Tuning::standard();
        for root in 0..12 {
            for q in [
                ChordQuality::Major,
                ChordQuality::Minor,
                ChordQuality::Dom7,
                ChordQuality::Sus2,
                ChordQuality::Add9,
                ChordQuality::Maj9,
                ChordQuality::Min11,
            ] {
                let c = chord(root, q);
                let v = neck_shapes(&c, &t);
                assert!(!v.is_empty(), "{}: no full shapes", c.pretty());
                for x in &v {
                    assert!(x.shape.check(&c, &t).ok(), "{} {}", c.pretty(), x.shape);
                    assert_eq!(
                        x.shape.bass(&t).unwrap().pc(),
                        c.root.pc(),
                        "{} root in bass",
                        c.pretty()
                    );
                    assert!(x.shape.strings_played() >= 4);
                }
                if matches!(q, ChordQuality::Major | ChordQuality::Minor) {
                    // At least the five CAGED positions' worth of places on the neck.
                    assert!(v.len() >= 5, "{}: only {} neck shapes", c.pretty(), v.len());
                }
            }
        }
    }

    #[test]
    fn familiar_shapes_are_found() {
        let t = Tuning::standard();
        let shapes = |c: Chord| {
            neck_shapes(&c, &t)
                .into_iter()
                .map(|v| v.shape.fret_string())
                .collect::<Vec<_>>()
        };
        let c = shapes(chord(0, ChordQuality::Major));
        assert!(
            c.contains(&"x32010".to_string()) || c.contains(&"x32013".to_string()),
            "{c:?}"
        );
        let f = shapes(chord(5, ChordQuality::Major));
        assert!(f.contains(&"133211".to_string()), "{f:?}");
        let am = shapes(chord(9, ChordQuality::Minor));
        assert!(am.contains(&"x02210".to_string()), "{am:?}");
    }

    #[test]
    fn fingers_are_playable() {
        // F barre needs finger 1 across fret 1.
        let f = assign_fingers(&[Some(1), Some(3), Some(3), Some(2), Some(1), Some(1)]).unwrap();
        assert_eq!(f, [Some(1), Some(3), Some(4), Some(2), Some(1), Some(1)]);
        // A five-fret stretch is out.
        assert!(assign_fingers(&[Some(1), Some(5), None, None, None, None]).is_none());
    }
}
