//! Standard notation: a treble-clef staff with key signature, noteheads, stems, flags,
//! ledger lines, and accidentals. Guitar music is written one octave higher than it
//! sounds, so the notes sit in the middle of the staff.
//!
//! The staff lays notes out with the same spacing as `TabStaff`, so the two line up when
//! drawn one above the other.

use crate::theme;
use content::TabNote;
use egui::{Align2, Color32, FontId, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};
use theory::{Key, NoteName, Spelling, Tuning, string_index};

/// Staff steps (lines and spaces) up from the bottom line E4 for the sharps and flats of a
/// key signature, in the order they're written.
const SHARP_STEPS: [i32; 7] = [8, 5, 9, 6, 3, 7, 4]; // F C G D A E B
const FLAT_STEPS: [i32; 7] = [4, 7, 3, 6, 2, 5, 1]; // B E A D G C F

/// Space between staff lines.
pub const GAP: f32 = 12.0;
/// Room for the clef and key signature before the first note (shared with the tab row).
pub fn lead(key: Option<&Key>) -> f32 {
    let accidentals = key.map_or(0, |k| k.signature().unsigned_abs() as usize);
    44.0 + accidentals as f32 * 10.0
}

/// How each written note is placed: staff step (0 = bottom line), and accidental to print.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub step: i32,
    /// Accidental to draw (None = nothing, Some(0) = natural sign).
    pub accidental: Option<i8>,
}

/// Spells and places a sounding pitch on the treble staff (written an octave up).
pub fn place(midi: i32, key: Option<&Key>) -> (NoteName, i32) {
    let written = midi + 12;
    let pc = written.rem_euclid(12) as theory::PitchClass;
    let name = key
        .and_then(|k| k.scale().notes().into_iter().find(|n| n.pc() == pc))
        .unwrap_or_else(|| {
            let flats = key.is_some_and(|k| k.signature() < 0);
            NoteName::from_pc(
                pc,
                if flats {
                    Spelling::Flats
                } else {
                    Spelling::Sharps
                },
            )
        });
    let natural = written - name.accidental as i32;
    let octave = natural.div_euclid(12) - 1;
    let diatonic = octave * 7 + name.letter.index() as i32;
    // E4 (bottom line) = octave 4, letter E (index 2).
    (name, diatonic - (4 * 7 + 2))
}

/// The accidental each letter carries in a key signature.
fn signature_accidental(key: Option<&Key>, letter: theory::Letter) -> i8 {
    key.and_then(|k| k.scale().notes().into_iter().find(|n| n.letter == letter))
        .map_or(0, |n| n.accidental)
}

/// Places a melody, deciding which accidentals to print (they last until the bar line).
pub fn place_all(midis: &[(i32, f32)], key: Option<&Key>, bar: f32) -> Vec<Placed> {
    let mut out = Vec::new();
    let mut in_bar: std::collections::HashMap<i32, i8> = Default::default();
    let mut beat = 0.0f32;
    for &(m, beats) in midis {
        if beat > 0.0 && (beat % bar).abs() < 1e-3 {
            in_bar.clear();
        }
        let (name, step) = place(m, key);
        let current = in_bar
            .get(&step)
            .copied()
            .unwrap_or_else(|| signature_accidental(key, name.letter));
        let accidental = (current != name.accidental).then_some(name.accidental);
        in_bar.insert(step, name.accidental);
        out.push(Placed { step, accidental });
        beat += beats;
    }
    out
}

pub struct StaffNotation<'a> {
    pub notes: &'a [TabNote],
    pub tuning: &'a Tuning,
    pub key: Option<&'a Key>,
    pub current: Option<usize>,
    pub width: f32,
    /// Beats in a bar.
    pub bar: f32,
}

impl StaffNotation<'_> {
    pub fn show(self, ui: &mut Ui) -> Response {
        let top_pad = 34.0; // room for high notes and ledger lines
        let h = top_pad + GAP * 4.0 + 40.0;
        let (response, painter) = ui.allocate_painter(Vec2::new(self.width, h), Sense::hover());
        let names: Vec<String> = self
            .notes
            .iter()
            .map(|n| {
                let m = self
                    .tuning
                    .pitch(string_index(n.string.clamp(1, 6)), n.fret)
                    .0;
                place(m, self.key).0.pretty()
            })
            .collect();
        crate::widgets::describe::set(
            &response,
            format!("Standard notation, treble clef: {}.", names.join(", ")),
        );
        let r = response.rect;
        let left = r.left() + lead(self.key) - 10.0;
        let right = r.right() - 10.0;
        let bottom = r.top() + top_pad + GAP * 4.0;
        let y_of = |step: i32| bottom - step as f32 * GAP / 2.0;
        let ink = Stroke::new(1.2, theme::INK);

        for line in 0..5 {
            let y = y_of(line * 2);
            painter.line_segment(
                [Pos2::new(r.left() + 4.0, y), Pos2::new(right, y)],
                Stroke::new(1.0, theme::SOFT_INK),
            );
        }
        painter.line_segment(
            [Pos2::new(right, y_of(0)), Pos2::new(right, y_of(8))],
            Stroke::new(2.0, theme::INK),
        );
        treble_clef(&painter, Pos2::new(r.left() + 16.0, y_of(2)));

        // Key signature.
        if let Some(k) = self.key {
            let n = k.signature();
            let (steps, glyph) = if n >= 0 {
                (&SHARP_STEPS, "♯")
            } else {
                (&FLAT_STEPS, "♭")
            };
            for (i, step) in steps.iter().take(n.unsigned_abs() as usize).enumerate() {
                painter.text(
                    Pos2::new(r.left() + 38.0 + i as f32 * 10.0, y_of(*step)),
                    Align2::CENTER_CENTER,
                    theme::glyphs(glyph),
                    FontId::proportional(GAP * 1.6),
                    theme::INK,
                );
            }
        }

        // Notes, spaced exactly like the tab underneath.
        let midis: Vec<(i32, f32)> = self
            .notes
            .iter()
            .map(|n| {
                (
                    self.tuning
                        .pitch(string_index(n.string.clamp(1, 6)), n.fret)
                        .0,
                    n.beats,
                )
            })
            .collect();
        let placed = place_all(&midis, self.key, self.bar);
        let space = |beats: f32| beats.max(0.75);
        let total: f32 = self
            .notes
            .iter()
            .map(|n| space(n.beats))
            .sum::<f32>()
            .max(1.0);
        let usable = right - left - 30.0;
        let mut x = left + 22.0;
        let mut beat_pos = 0.0f32;
        for (i, (n, p)) in self.notes.iter().zip(&placed).enumerate() {
            if i > 0 && beat_pos > 0.0 && (beat_pos % self.bar).abs() < 1e-3 {
                let bx = x - usable * 0.25 / total * 2.0;
                painter.line_segment([Pos2::new(bx, y_of(0)), Pos2::new(bx, y_of(8))], ink);
            }
            let y = y_of(p.step);
            let color = if self.current == Some(i) {
                theme::VERMILLION
            } else {
                theme::INK
            };
            if self.current == Some(i) {
                painter.circle_filled(
                    Pos2::new(x, y),
                    GAP * 1.1,
                    theme::YELLOW.gamma_multiply(0.6),
                );
            }
            // Ledger lines above and below the staff.
            for step in (-12..=-2).rev().step_by(2).filter(|s| *s >= p.step) {
                painter.line_segment(
                    [
                        Pos2::new(x - 10.0, y_of(step)),
                        Pos2::new(x + 10.0, y_of(step)),
                    ],
                    ink,
                );
            }
            for step in (10..=20).step_by(2).filter(|s| *s <= p.step) {
                painter.line_segment(
                    [
                        Pos2::new(x - 10.0, y_of(step)),
                        Pos2::new(x + 10.0, y_of(step)),
                    ],
                    ink,
                );
            }
            if let Some(acc) = p.accidental {
                let glyph = match acc {
                    0 => "♮",
                    a if a > 0 => "♯",
                    _ => "♭",
                };
                painter.text(
                    Pos2::new(x - 15.0, y),
                    Align2::CENTER_CENTER,
                    theme::glyphs(glyph),
                    FontId::proportional(GAP * 1.5),
                    color,
                );
            }
            note_head(&painter, Pos2::new(x, y), n.beats, p.step, color);
            beat_pos += n.beats;
            x += usable * space(n.beats) / total;
        }
        response
    }
}

/// A notehead with its stem, flags, and dot, chosen from the note's length in beats.
fn note_head(painter: &egui::Painter, c: Pos2, beats: f32, step: i32, color: Color32) {
    let dotted = [0.75, 1.5, 3.0].iter().any(|d| (beats - d).abs() < 1e-3);
    let base = if dotted { beats / 1.5 } else { beats };
    let hollow = base >= 2.0;
    let rx = GAP * 0.62;
    let ry = GAP * 0.45;
    // A slightly tilted oval made from points.
    let pts: Vec<Pos2> = (0..20)
        .map(|i| {
            let t = i as f32 / 20.0 * std::f32::consts::TAU;
            let (x, y) = (rx * t.cos(), ry * t.sin());
            let (s, co) = (-0.35f32).sin_cos();
            Pos2::new(c.x + x * co - y * s, c.y + x * s + y * co)
        })
        .collect();
    if hollow {
        painter.add(Shape::closed_line(pts, Stroke::new(1.8, color)));
    } else {
        painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    }
    if dotted {
        painter.circle_filled(
            Pos2::new(
                c.x + rx + 6.0,
                c.y - if step % 2 == 0 { GAP / 2.0 } else { 0.0 },
            ),
            2.2,
            color,
        );
    }
    if base >= 4.0 {
        return; // whole note: no stem
    }
    // Stems go up for notes below the middle line, down otherwise.
    let up = step < 4;
    let stem_x = if up { c.x + rx - 1.0 } else { c.x - rx + 1.0 };
    let tip = if up { c.y - GAP * 3.3 } else { c.y + GAP * 3.3 };
    painter.line_segment(
        [Pos2::new(stem_x, c.y), Pos2::new(stem_x, tip)],
        Stroke::new(1.4, color),
    );
    let flags = if base <= 0.25 {
        2
    } else if base <= 0.5 {
        1
    } else {
        0
    };
    for f in 0..flags {
        let y0 = if up {
            tip + f as f32 * 7.0
        } else {
            tip - f as f32 * 7.0
        };
        let dir = if up { 1.0 } else { -1.0 };
        painter.add(Shape::CubicBezier(
            egui::epaint::CubicBezierShape::from_points_stroke(
                [
                    Pos2::new(stem_x, y0),
                    Pos2::new(stem_x + 2.0, y0 + dir * 8.0),
                    Pos2::new(stem_x + 11.0, y0 + dir * 10.0),
                    Pos2::new(stem_x + 8.0, y0 + dir * 20.0),
                ],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.8, color),
            ),
        ));
    }
}

/// A simple treble (G) clef curling around the G line.
fn treble_clef(painter: &egui::Painter, g: Pos2) {
    let s = GAP;
    let stroke = Stroke::new(2.0, theme::INK);
    let p = |x: f32, y: f32| Pos2::new(g.x + x * s, g.y + y * s);
    let curves = [
        // inner curl around the G line
        [p(0.15, 0.1), p(-0.6, 0.0), p(-0.5, -1.0), p(0.3, -0.9)],
        [p(0.3, -0.9), p(1.0, -0.8), p(1.0, 0.6), p(0.0, 0.7)],
        [p(0.0, 0.7), p(-1.2, 0.8), p(-1.0, -1.2), p(0.0, -2.0)],
        // up to the top loop
        [p(0.0, -2.0), p(0.8, -2.7), p(0.8, -3.8), p(0.2, -3.6)],
        // down the spine
        [p(0.2, -3.6), p(-0.4, -3.2), p(0.1, 1.0), p(0.3, 2.0)],
        // tail hook
        [p(0.3, 2.0), p(0.4, 2.6), p(-0.4, 2.8), p(-0.4, 2.2)],
    ];
    for pts in curves {
        painter.add(Shape::CubicBezier(
            egui::epaint::CubicBezierShape::from_points_stroke(
                pts,
                false,
                Color32::TRANSPARENT,
                stroke,
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_land_on_the_right_lines() {
        // Guitar sounds an octave lower: open high E (E4 sounding) is written E5, top space.
        assert_eq!(place(64, None).1, 7);
        // Open G string (G3) is written G4: second line.
        assert_eq!(place(55, None).1, 2);
        // Low E string (E2) is written E3: three ledger lines below.
        assert_eq!(place(40, None).1, -7);
        // Middle C written (sounding C3) sits on one ledger line below.
        assert_eq!(place(48, None).1, -2);
    }

    #[test]
    fn accidentals_follow_the_key_and_the_bar() {
        let g = Key::major("G".parse().unwrap());
        // F♯ in G: no sign needed. F natural: natural sign.
        let placed = place_all(&[(54, 1.0), (53, 1.0)], Some(&g), 4.0);
        assert_eq!(placed[0].accidental, None);
        assert_eq!(placed[1].accidental, Some(0));
        // In C, a sharp carries through the bar but is shown again after the bar line.
        let placed = place_all(&[(54, 1.0), (54, 3.0), (54, 1.0)], None, 4.0);
        assert_eq!(placed[0].accidental, Some(1));
        assert_eq!(placed[1].accidental, None);
        assert_eq!(placed[2].accidental, Some(1));
        // In F, B♭ is spelled with a flat letter B.
        let f = Key::major("F".parse().unwrap());
        assert_eq!(place(58, Some(&f)).0.pretty(), "B♭");
    }
}
