//! Plain-text descriptions of diagrams, generated from the same data the pictures are
//! drawn from. Screen readers (through AccessKit) read these, and tests can find
//! diagrams by them.

use super::Dot;
use content::{Stroke, StrumPattern, TabNote};
use egui::{Response, WidgetInfo, WidgetType};
use theory::{Chord, ChordShape, NoteName, Spelling, Tuning, string_number};

/// Attaches a description to a painted diagram.
pub fn set(response: &Response, text: String) {
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, text.clone()));
}

fn note(tuning: &Tuning, string: usize, fret: u8) -> String {
    NoteName::from_pc(tuning.pitch(string, fret).pc(), Spelling::Sharps).pretty()
}

/// "C major chord. String 5: fret 3, finger 3 (C). … Strings 6 muted."
pub fn chord(shape: &ChordShape, chord: Option<&Chord>, tuning: &Tuning) -> String {
    let mut parts = vec![match chord {
        Some(c) => format!("{} chord diagram.", c.pretty()),
        None => "Chord diagram.".to_string(),
    }];
    let mut muted = Vec::new();
    let mut open = Vec::new();
    for s in 0..6 {
        match shape.frets[s] {
            None => muted.push(string_number(s).to_string()),
            Some(0) => open.push(format!("{} ({})", string_number(s), note(tuning, s, 0))),
            Some(f) => parts.push(format!(
                "String {}: fret {f}{} ({}).",
                string_number(s),
                shape.fingers[s]
                    .map(|g| format!(", finger {g}"))
                    .unwrap_or_default(),
                note(tuning, s, f)
            )),
        }
    }
    if !open.is_empty() {
        parts.push(format!("Open: string {}.", open.join(", ")));
    }
    if !muted.is_empty() {
        parts.push(format!("Don't play string {}.", muted.join(", ")));
    }
    parts.join(" ")
}

/// "Neck map, frets 3 to 10. 12 notes marked: string 6 fret 5 R, …"
pub fn neck(dots: &[Dot], first: u8, last: u8) -> String {
    let lit: Vec<&Dot> = dots.iter().filter(|d| !d.faint).collect();
    let mut text = format!(
        "Neck map, frets {first} to {last}. {} notes marked",
        lit.len()
    );
    if lit.is_empty() {
        return text + ".";
    }
    let list: Vec<String> = lit
        .iter()
        .take(40)
        .map(|d| {
            let label = if d.label.is_empty() {
                String::new()
            } else {
                format!(" {}", d.label)
            };
            format!("string {} fret {}{label}", string_number(d.string), d.fret)
        })
        .collect();
    text.push_str(": ");
    text.push_str(&list.join(", "));
    if lit.len() > 40 {
        text.push_str(", and more");
    }
    text + "."
}

/// "Tab: string 3 fret 5, string 3 fret 7 hammer-on, …"
pub fn tab(notes: &[TabNote]) -> String {
    let list: Vec<String> = notes
        .iter()
        .map(|n| {
            let tech = match n.tech {
                Some(content::Tech::Hammer) => " hammer-on",
                Some(content::Tech::Pull) => " pull-off",
                Some(content::Tech::Slide) => " slide",
                Some(content::Tech::Bend(1)) => " bend half step",
                Some(content::Tech::Bend(_)) => " bend whole step",
                Some(content::Tech::Vibrato) => " vibrato",
                None => "",
            };
            let len = if (n.beats - 1.0).abs() < 1e-3 {
                String::new()
            } else {
                format!(" ({} beats)", n.beats)
            };
            format!("string {} fret {}{tech}{len}", n.string, n.fret)
        })
        .collect();
    format!("Tab, {} notes: {}.", notes.len(), list.join(", "))
}

/// "Strum pattern Old Faithful: 1 down, & rest, 2 down, …"
pub fn strum(pattern: StrumPattern) -> String {
    let words: Vec<String> = pattern
        .slots()
        .iter()
        .zip(pattern.counts())
        .map(|(s, c)| {
            let what = match s {
                Stroke::Down => "down".to_string(),
                Stroke::Up => "up".to_string(),
                Stroke::Ghost => "rest".to_string(),
                Stroke::Bass(0) => "bass note".to_string(),
                Stroke::Bass(_) => "other bass note".to_string(),
                Stroke::Pluck(2) => "index finger".to_string(),
                Stroke::Pluck(3) => "middle finger".to_string(),
                Stroke::Pluck(_) => "ring finger".to_string(),
                Stroke::Boogie(0) => "root and 5th".to_string(),
                Stroke::Boogie(_) => "root and 6th".to_string(),
            };
            format!("{c} {what}")
        })
        .collect();
    format!("Strum pattern {}: {}.", pattern.name(), words.join(", "))
}
