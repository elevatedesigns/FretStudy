//! Pictures for music-theory lessons: piano strip, musical alphabet, scale staircase,
//! chord-number wheel, and note lengths.

use crate::theme;
use crate::widgets::{Dot, DotShape, LabelMode};
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2,
};
use theory::{ChordQuality, Key, NoteName, Role, Scale, ScaleKind, Tuning};

/// One octave of piano keys (C to C) with the E–F and B–C half steps highlighted.
pub fn piano(ui: &mut Ui, width: f32) -> Response {
    let white_w = width / 8.0;
    let h = white_w * 3.0;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, h + 30.0), Sense::hover());
    super::describe::set(&response, "Piano keys from C to C. White keys C D E F G A B C. Black keys are the sharps and flats. E to F and B to C have no black key between them: they are half steps.".into());
    let r = response.rect;
    let whites = ["C", "D", "E", "F", "G", "A", "B", "C"];
    for (i, n) in whites.iter().enumerate() {
        let key = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * white_w, r.top()),
            Vec2::new(white_w - 2.0, h),
        );
        painter.rect(
            key,
            CornerRadius::same(4),
            Color32::WHITE,
            Stroke::new(1.5, theme::INK.gamma_multiply(0.6)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            Pos2::new(key.center().x, key.bottom() - 16.0),
            Align2::CENTER_CENTER,
            *n,
            theme::heading(18.0),
            theme::INK,
        );
    }
    // Black keys sit between C–D, D–E, F–G, G–A, A–B (not E–F or B–C).
    let blacks = [
        (0, "C♯\nD♭"),
        (1, "D♯\nE♭"),
        (3, "F♯\nG♭"),
        (4, "G♯\nA♭"),
        (5, "A♯\nB♭"),
    ];
    for (i, label) in blacks {
        let x = r.left() + (i as f32 + 1.0) * white_w - white_w * 0.3;
        let key = Rect::from_min_size(Pos2::new(x, r.top()), Vec2::new(white_w * 0.56, h * 0.6));
        painter.rect_filled(key, CornerRadius::same(3), theme::INK);
        painter.text(
            Pos2::new(key.center().x, key.bottom() - 18.0),
            Align2::CENTER_CENTER,
            theme::glyphs(label),
            FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
    // Highlight the natural half steps.
    for i in [2usize, 6] {
        let x = r.left() + (i as f32 + 1.0) * white_w - 1.0;
        painter.line_segment(
            [Pos2::new(x, r.top() + h * 0.62), Pos2::new(x, r.top() + h)],
            Stroke::new(4.0, theme::ORANGE),
        );
    }
    painter.text(Pos2::new(r.left(), r.top() + h + 16.0), Align2::LEFT_CENTER, "Orange lines: E–F and B–C are next-door neighbors (a half step) with no black key between.", FontId::proportional(14.0), theme::SOFT_INK);
    response
}

fn letter_color(letter: char) -> Color32 {
    match letter {
        'A' => theme::VERMILLION,
        'B' => theme::ORANGE,
        'C' => theme::YELLOW,
        'D' => theme::GREEN,
        'E' => theme::SKY_BLUE,
        'F' => theme::BLUE,
        _ => theme::PURPLE,
    }
}

/// The musical alphabet climbing and repeating: A B C D E F G A B C…
pub fn alphabet(ui: &mut Ui, width: f32) -> Response {
    let letters: Vec<char> = "ABCDEFGABCDEFGA".chars().collect();
    let w = width / letters.len() as f32;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, w * 2.6 + 24.0), Sense::hover());
    super::describe::set(
        &response,
        "The musical alphabet climbing: A B C D E F G, then A again an octave higher.".into(),
    );
    let r = response.rect;
    for (i, l) in letters.iter().enumerate() {
        // Each tile sits a little higher: notes climb as you go up the alphabet.
        let y = r.top() + w * 1.6 - i as f32 * w * 0.1;
        let tile = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * w, y - w * 0.5),
            Vec2::new(w - 3.0, w * 0.9),
        );
        let c = letter_color(*l);
        painter.rect_filled(tile, CornerRadius::same(6), c);
        painter.text(
            tile.center(),
            Align2::CENTER_CENTER,
            l.to_string(),
            theme::heading(w * 0.45),
            theme::text_on(c),
        );
    }
    // Octave arcs from A to the next A.
    for (from, to) in [(0usize, 7usize), (7, 14)] {
        let a = Pos2::new(
            r.left() + (from as f32 + 0.5) * w,
            r.top() + w * 2.25 - from as f32 * w * 0.1,
        );
        let b = Pos2::new(
            r.left() + (to as f32 + 0.5) * w,
            r.top() + w * 2.25 - to as f32 * w * 0.1,
        );
        painter.line_segment([a, b], Stroke::new(2.0, theme::VERMILLION));
        painter.text(
            a.lerp(b, 0.5) + Vec2::new(0.0, 12.0),
            Align2::CENTER_CENTER,
            "octave",
            FontId::proportional(13.0),
            theme::VERMILLION,
        );
    }
    response
}

/// The major-scale staircase from a root: tall steps are whole steps (2 frets), short ones
/// half steps (1 fret). Also shows the frets on the matching open string, if there is one.
pub fn scale_steps(ui: &mut Ui, root: NoteName, width: f32) -> Response {
    let scale = Scale::new(root, ScaleKind::Major);
    let mut notes = scale.notes();
    notes.push(root);
    let steps = ScaleKind::Major.step_pattern();
    let col = width / 8.0;
    let unit = 16.0; // height of one half step
    let h = unit * 12.0 + 70.0;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, h), Sense::hover());
    super::describe::set(
        &response,
        format!(
            "{} major scale as a staircase: {}. Steps: {}.",
            root.pretty(),
            notes
                .iter()
                .map(|n| n.pretty())
                .collect::<Vec<_>>()
                .join(" "),
            steps.join(" ")
        ),
    );
    let r = response.rect;
    let base = r.bottom() - 40.0;
    let mut height = 0.0;
    for (i, n) in notes.iter().enumerate() {
        let x = r.left() + i as f32 * col;
        let step = Rect::from_min_max(
            Pos2::new(x, base - height - unit * 1.6),
            Pos2::new(x + col - 4.0, base - height),
        );
        let fill = if i == 0 || i == 7 {
            theme::VERMILLION
        } else {
            theme::SKY_BLUE
        };
        painter.rect_filled(step, CornerRadius::same(5), fill);
        painter.text(
            step.center(),
            Align2::CENTER_CENTER,
            theme::glyphs(&n.pretty()),
            theme::heading(18.0),
            theme::text_on(fill),
        );
        painter.text(
            Pos2::new(step.center().x, base + 14.0),
            Align2::CENTER_CENTER,
            format!("{}", i + 1),
            FontId::proportional(13.0),
            theme::SOFT_INK,
        );
        if i < 7 {
            let s = steps[i];
            // W/H between the step numbers: 1 W 2 W 3 H 4 …
            painter.text(
                Pos2::new(x + col - 2.0, base + 14.0),
                Align2::CENTER_CENTER,
                s,
                theme::heading(15.0),
                if s == "H" {
                    theme::ORANGE
                } else {
                    theme::GREEN
                },
            );
            height += if s == "H" { unit } else { unit * 2.0 };
        }
    }
    painter.text(
        Pos2::new(r.left(), r.bottom() - 8.0),
        Align2::LEFT_CENTER,
        "W = whole step (2 frets) · H = half step (1 fret)",
        FontId::proportional(14.0),
        theme::SOFT_INK,
    );
    // The same recipe on one string.
    let t = Tuning::standard();
    if let Some(s) = (0..6).rev().find(|s| t.open[*s].pc() == root.pc()) {
        let frets = [0, 2, 4, 5, 7, 9, 11, 12]
            .map(|f| f.to_string())
            .join(" – ");
        ui.label(
            RichText::new(format!(
                "On string {} (open {}): frets {frets}",
                theory::string_number(s),
                theme::glyphs(&root.pretty())
            ))
            .size(16.0),
        );
    }
    response
}

/// The chords of a key as numbered tiles: triads or 7th chords, major or minor key.
pub fn key_chords(key: NoteName, sevenths: bool, minor: bool) -> Vec<theory::key::DiatonicChord> {
    let k = if minor {
        Key::minor(key)
    } else {
        Key::major(key)
    };
    if sevenths { k.sevenths() } else { k.triads() }
}

/// The 7 chords of a key as numbered tiles (warm = major, red = dominant 7th, cool = minor,
/// gray = diminished). Returns the tile clicked, if any.
pub fn number_wheel(
    ui: &mut Ui,
    chords: &[theory::key::DiatonicChord],
    width: f32,
) -> Option<usize> {
    let w = width / 7.0;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, w * 1.25), Sense::click());
    let names: Vec<String> = chords
        .iter()
        .map(|d| format!("{} {}", d.number, d.chord.pretty()))
        .collect();
    super::describe::set(
        &response,
        format!("Chords of the key: {}.", names.join(", ")),
    );
    let r = response.rect;
    let mut clicked = None;
    for (i, d) in chords.iter().enumerate() {
        let tile = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * w, r.top()),
            Vec2::new(w - 6.0, w * 1.2),
        );
        let fill = match d.chord.quality {
            ChordQuality::Major | ChordQuality::Maj7 => theme::ORANGE,
            ChordQuality::Dom7 => theme::VERMILLION,
            ChordQuality::Minor | ChordQuality::Min7 => theme::SKY_BLUE,
            _ => theme::LIGHT_GRAY,
        };
        let star = matches!(d.number, 1 | 4 | 5);
        painter.rect_filled(tile, CornerRadius::same(10), fill);
        if star {
            painter.rect_stroke(
                tile,
                CornerRadius::same(10),
                Stroke::new(3.0, theme::INK),
                egui::StrokeKind::Inside,
            );
        }
        painter.text(
            Pos2::new(tile.center().x, tile.top() + w * 0.28),
            Align2::CENTER_CENTER,
            d.number.to_string(),
            theme::heading(w * 0.32),
            theme::text_on(fill),
        );
        painter.text(
            Pos2::new(tile.center().x, tile.top() + w * 0.62),
            Align2::CENTER_CENTER,
            theme::glyphs(&d.chord.pretty().replace("dim", "°")),
            theme::heading(
                w * if d.chord.pretty().chars().count() > 4 {
                    0.15
                } else {
                    0.2
                },
            ),
            theme::text_on(fill),
        );
        painter.text(
            Pos2::new(tile.center().x, tile.top() + w * 0.95),
            Align2::CENTER_CENTER,
            &d.numeral,
            FontId::proportional(w * 0.15),
            theme::text_on(fill),
        );
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
            && tile.contains(p)
        {
            clicked = Some(i);
        }
    }
    let has_dom = chords.iter().any(|d| d.chord.quality == ChordQuality::Dom7);
    ui.label(
        RichText::new(if has_dom {
            "Orange = major 7 · Red = dominant 7 · Blue = minor 7 · Gray = half-diminished · Click a chord to hear it."
        } else {
            "Orange = major · Blue = minor · Gray = diminished · Outlined = the magic 1, 4, and 5. Click a chord to hear it."
        })
        .small()
        .color(theme::SOFT_INK),
    );
    clicked
}

/// Note and rest lengths on a 4-beat timeline.
pub fn durations(ui: &mut Ui, width: f32) -> Response {
    let rows: [(&str, f32, Color32); 4] = [
        ("Whole note / whole rest: 4 beats", 4.0, theme::VERMILLION),
        ("Half: 2 beats", 2.0, theme::ORANGE),
        ("Quarter: 1 beat", 1.0, theme::GREEN),
        ("Eighth: ½ beat", 0.5, theme::BLUE),
    ];
    let label_w = 250.0;
    let bar_w = (width - label_w).max(200.0);
    let row_h = 34.0;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, row_h * 5.0), Sense::hover());
    super::describe::set(&response, "Note lengths: whole note 4 beats, half note 2 beats, quarter note 1 beat, eighth note half a beat.".into());
    let r = response.rect;
    let beat_w = bar_w / 4.0;
    for b in 0..4 {
        painter.text(
            Pos2::new(
                r.left() + label_w + beat_w * (b as f32 + 0.5),
                r.top() + 10.0,
            ),
            Align2::CENTER_CENTER,
            (b + 1).to_string(),
            theme::heading(16.0),
            theme::INK,
        );
    }
    for (i, (label, len, color)) in rows.iter().enumerate() {
        let y = r.top() + row_h * (i as f32 + 1.0);
        painter.text(
            Pos2::new(r.left(), y + row_h * 0.4),
            Align2::LEFT_CENTER,
            *label,
            FontId::proportional(15.0),
            theme::INK,
        );
        let n = (4.0 / len) as usize;
        for k in 0..n {
            let block = Rect::from_min_size(
                Pos2::new(r.left() + label_w + k as f32 * beat_w * len, y + 4.0),
                Vec2::new(beat_w * len - 4.0, row_h - 10.0),
            );
            painter.rect_filled(block, CornerRadius::same(6), *color);
        }
    }
    response
}

/// A scale on the neck as dots in note-job colors: roots are squares, the blues note a
/// diamond. With `shape` (1–5, pentatonic scales only) the notes outside that box are faint.
pub fn scale_dots(
    tuning: &Tuning,
    root: NoteName,
    kind: ScaleKind,
    shape: Option<u8>,
    frets: (u8, u8),
    label_mode: LabelMode,
) -> Vec<Dot> {
    let scale = Scale::new(root, kind);
    let names = scale.notes();
    let in_box: Option<Vec<(usize, u8)>> = shape.filter(|s| (1..=5).contains(s)).and_then(|s| {
        let minor_root = match kind {
            ScaleKind::MinorPentatonic | ScaleKind::Blues => root.pc(),
            ScaleKind::MajorPentatonic => (root.pc() + 9) % 12,
            _ => return None,
        };
        let notes = theory::shapes::pentatonic_box(minor_root, s as usize, frets.0, tuning);
        let mut cells: Vec<(usize, u8)> = notes.iter().map(|n| (n.string, n.fret)).collect();
        if kind == ScaleKind::Blues {
            // The blue note sits next to the box's 4ths and 5ths: include it where it fits.
            let lo = cells.iter().map(|c| c.1).min().unwrap_or(0);
            let hi = cells.iter().map(|c| c.1).max().unwrap_or(0);
            let blue = (root.pc() + 6) % 12;
            for s in 0..6 {
                for f in lo..=hi {
                    if tuning.pitch(s, f).pc() == blue {
                        cells.push((s, f));
                    }
                }
            }
        }
        Some(cells)
    });
    let mut dots = Vec::new();
    for s in 0..6 {
        for f in frets.0..=frets.1 {
            let pc = tuning.pitch(s, f).pc();
            let Some(role) = scale.role_of(pc) else {
                continue;
            };
            let label = match label_mode {
                LabelMode::Note => names
                    .iter()
                    .find(|n| n.pc() == pc)
                    .map(|n| theme::glyphs(&n.pretty()))
                    .unwrap_or_default(),
                _ => theme::glyphs(&scale.label_of(pc).unwrap_or_default()),
            };
            dots.push(Dot {
                string: s,
                fret: f,
                fill: theme::role_color(role),
                label,
                shape: match role {
                    Role::Root => DotShape::Square,
                    Role::BlueNote => DotShape::Diamond,
                    _ => DotShape::Circle,
                },
                faint: in_box.as_ref().is_some_and(|b| !b.contains(&(s, f))),
            });
        }
    }
    dots
}

/// The 12-bar blues as a 3-row chart of bars: 1 1 1 1 / 4 4 1 1 / 5 4 1 5.
/// `playing` lights one bar. Returns the bar clicked, if any.
pub const TWELVE_BAR: [u8; 12] = [1, 1, 1, 1, 4, 4, 1, 1, 5, 4, 1, 5];

pub fn twelve_bar(ui: &mut Ui, key: NoteName, playing: Option<usize>, width: f32) -> Response {
    let cell_w = width / 4.0;
    let cell_h = 64.0;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(width, cell_h * 3.0 + 6.0), Sense::hover());
    super::describe::set(
        &response,
        format!(
            "12-bar blues in {}: {}.",
            key.pretty(),
            TWELVE_BAR.map(|n| n.to_string()).join(" ")
        ),
    );
    let r = response.rect;
    let major = Key::major(key);
    for (i, number) in TWELVE_BAR.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(
                r.left() + (i % 4) as f32 * cell_w,
                r.top() + (i / 4) as f32 * cell_h,
            ),
            Vec2::new(cell_w - 6.0, cell_h - 6.0),
        );
        let fill = match number {
            1 => theme::ORANGE,
            4 => theme::GREEN,
            _ => theme::BLUE,
        };
        let fill = if playing == Some(i) {
            theme::YELLOW
        } else {
            fill
        };
        painter.rect_filled(cell, CornerRadius::same(10), fill);
        let chord = major.chord_number(*number);
        painter.text(
            Pos2::new(cell.left() + 12.0, cell.center().y),
            Align2::LEFT_CENTER,
            number.to_string(),
            theme::heading(28.0),
            theme::text_on(fill),
        );
        painter.text(
            Pos2::new(cell.right() - 12.0, cell.center().y),
            Align2::RIGHT_CENTER,
            theme::glyphs(&format!("{}7", chord.root.pretty())),
            theme::heading(20.0),
            theme::text_on(fill),
        );
        painter.text(
            Pos2::new(cell.left() + 6.0, cell.top() + 9.0),
            Align2::LEFT_CENTER,
            format!("bar {}", i + 1),
            FontId::proportional(10.0),
            theme::text_on(fill),
        );
    }
    response
}

/// Two scales from one root on the neck: notes in both are gray, notes only in `a` are sky
/// blue circles, notes only in `b` are blue diamonds. Roots are squares.
pub fn compare_dots(
    tuning: &Tuning,
    root: NoteName,
    a: ScaleKind,
    b: ScaleKind,
    frets: (u8, u8),
) -> Vec<Dot> {
    let (sa, sb) = (Scale::new(root, a), Scale::new(root, b));
    let mut dots = Vec::new();
    for s in 0..6 {
        for f in frets.0..=frets.1 {
            let pc = tuning.pitch(s, f).pc();
            let (in_a, in_b) = (sa.contains(pc), sb.contains(pc));
            if !in_a && !in_b {
                continue;
            }
            let label = sa
                .label_of(pc)
                .or_else(|| sb.label_of(pc))
                .unwrap_or_default();
            let (fill, shape) = if pc == root.pc() {
                (theme::VERMILLION, DotShape::Square)
            } else if in_a && in_b {
                (theme::LIGHT_GRAY, DotShape::Circle)
            } else if in_a {
                (theme::SKY_BLUE, DotShape::Circle)
            } else {
                (theme::BLUE, DotShape::Diamond)
            };
            dots.push(Dot {
                string: s,
                fret: f,
                fill,
                label: theme::glyphs(&label),
                shape,
                faint: false,
            });
        }
    }
    dots
}
