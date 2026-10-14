//! The vertical chord box: nut on top, thickest string on the left (mirrored for lefties).

use super::{DiagramStyle, LabelMode, chord_dots, paint_dot};
use crate::theme;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use theory::{Chord, ChordShape, NoteName, Tuning};

pub struct ChordBox<'a> {
    pub shape: &'a ChordShape,
    pub chord: Option<&'a Chord>,
    pub tuning: &'a Tuning,
    pub style: DiagramStyle,
    pub width: f32,
    /// Per-string glow (0..1) while a string is ringing.
    pub glow: [f32; 6],
}

pub struct ChordBoxResponse {
    pub response: Response,
    /// A string the learner clicked (to hear it alone).
    pub clicked_string: Option<usize>,
}

impl<'a> ChordBox<'a> {
    pub fn new(shape: &'a ChordShape, chord: Option<&'a Chord>, tuning: &'a Tuning) -> Self {
        ChordBox {
            shape,
            chord,
            tuning,
            style: DiagramStyle::default(),
            width: 240.0,
            glow: [0.0; 6],
        }
    }

    pub fn style(mut self, style: DiagramStyle) -> Self {
        self.style = style;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn glow(mut self, glow: [f32; 6]) -> Self {
        self.glow = glow;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ChordBoxResponse {
        let w = self.width;
        let side = w * 0.16; // room for the "5fr" label
        let grid_w = w - side * 1.5;
        let sx = grid_w / 5.0;
        let frets_shown: u8 = {
            let lo = self.shape.lowest_fret().unwrap_or(1);
            let hi = self.shape.highest_fret().unwrap_or(1);
            if self.shape.fits_at_nut(4) {
                4
            } else {
                (hi - lo + 1).max(4)
            }
        };
        let base: u8 = if self.shape.fits_at_nut(4) {
            1
        } else {
            self.shape.lowest_fret().unwrap_or(1)
        };
        let sy = sx * 1.25;
        let marker_h = sx * 0.9;
        let label_h = sx * 0.85;
        let h = marker_h + sy * frets_shown as f32 + label_h + 6.0;

        let (response, painter) = ui.allocate_painter(Vec2::new(w, h), Sense::click());
        super::describe::set(
            &response,
            super::describe::chord(self.shape, self.chord, self.tuning),
        );
        let rect = response.rect;
        let left = rect.left() + side;
        let top = rect.top() + marker_h;
        let string_x = |s: usize| {
            let col = if self.style.left_handed { 5 - s } else { s };
            left + col as f32 * sx
        };
        let fret_y = |f: u8| top + (f - base) as f32 * sy + sy * 0.5; // center of the fret space

        // Fretboard wood
        let board = Rect::from_min_max(
            Pos2::new(left - sx * 0.3, top),
            Pos2::new(left + grid_w + sx * 0.3, top + sy * frets_shown as f32),
        );
        painter.rect_filled(board, CornerRadius::same(4), theme::WOOD);

        // Fret wires and nut
        for i in 0..=frets_shown {
            let y = top + i as f32 * sy;
            let (width, color) = if i == 0 && base == 1 {
                (sx * 0.22, theme::NUT)
            } else {
                (2.0, theme::FRET)
            };
            painter.line_segment(
                [Pos2::new(left, y), Pos2::new(left + grid_w, y)],
                Stroke::new(width, color),
            );
        }
        if base > 1 {
            painter.text(
                Pos2::new(
                    if self.style.left_handed {
                        left + grid_w + sx * 0.45
                    } else {
                        left - sx * 0.45
                    },
                    fret_y(base),
                ),
                if self.style.left_handed {
                    Align2::LEFT_CENTER
                } else {
                    Align2::RIGHT_CENTER
                },
                format!("{base}fr"),
                FontId::proportional(sx * 0.42),
                theme::SOFT_INK,
            );
        }
        // Strings (thicker for bass strings)
        for s in 0..6 {
            let x = string_x(s);
            let thickness = 1.0 + (5 - s) as f32 * 0.35;
            let glow = self.glow[s];
            let color = if glow > 0.0 {
                theme::STRING.lerp_to_gamma(theme::ORANGE, glow)
            } else {
                theme::STRING
            };
            painter.line_segment(
                [
                    Pos2::new(x, top),
                    Pos2::new(x, top + sy * frets_shown as f32),
                ],
                Stroke::new(thickness + glow * 1.5, color),
            );
        }

        let dots = chord_dots(self.shape, self.chord, self.tuning, &self.style);
        let radius = sx * 0.38;

        // Open (O) and muted (X) markers above the nut
        for s in 0..6 {
            let center = Pos2::new(string_x(s), rect.top() + marker_h * 0.5);
            match self.shape.frets[s] {
                None => {
                    let d = radius * 0.6;
                    let stroke = Stroke::new(2.2, theme::INK);
                    painter.line_segment(
                        [center + Vec2::new(-d, -d), center + Vec2::new(d, d)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(-d, d), center + Vec2::new(d, -d)],
                        stroke,
                    );
                }
                Some(0) => {
                    let ring = match self.style.color_mode {
                        super::ColorMode::Role => dots
                            .iter()
                            .find(|d| d.string == s)
                            .map(|d| d.fill)
                            .unwrap_or(theme::INK),
                        super::ColorMode::Finger => theme::INK,
                    };
                    painter.circle_stroke(center, radius * 0.62, Stroke::new(2.4, ring));
                }
                _ => {}
            }
        }

        // Barres (drawn under the dots)
        let in_finger_mode = self.style.color_mode == super::ColorMode::Finger;
        for barre in &self.shape.barres {
            if barre.fret < base || barre.fret >= base + frets_shown {
                continue;
            }
            let (x1, x2) = {
                let a = string_x(barre.from);
                let b = string_x(barre.to);
                (a.min(b), a.max(b))
            };
            let y = fret_y(barre.fret);
            let r = Rect::from_min_max(
                Pos2::new(x1 - radius, y - radius * 0.8),
                Pos2::new(x2 + radius, y + radius * 0.8),
            );
            let fill = if in_finger_mode {
                theme::finger_color(barre.finger)
            } else {
                theme::INK.gamma_multiply(0.25)
            };
            painter.rect_filled(r, CornerRadius::same((radius * 0.8) as u8), fill);
        }

        // Finger dots
        for dot in &dots {
            if dot.fret == 0 {
                continue;
            }
            let under_barre = self.shape.barres.iter().any(|b| {
                b.fret == dot.fret
                    && self.shape.fingers[dot.string] == Some(b.finger)
                    && (b.from..=b.to).contains(&dot.string)
            });
            let center = Pos2::new(string_x(dot.string), fret_y(dot.fret));
            if under_barre && in_finger_mode {
                // One label for the whole barre, at the thickest barred string.
                let barre = self
                    .shape
                    .barres
                    .iter()
                    .find(|b| b.fret == dot.fret)
                    .unwrap();
                if dot.string == barre.from && self.style.label_mode == LabelMode::Finger {
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        &dot.label,
                        FontId::proportional(radius * 1.1),
                        theme::text_on(dot.fill),
                    );
                } else if self.style.label_mode != LabelMode::Finger {
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        &dot.label,
                        FontId::proportional(radius * 0.85),
                        theme::text_on(dot.fill),
                    );
                }
                continue;
            }
            paint_dot(&painter, center, radius, dot, self.glow[dot.string]);
        }

        // Note names (or intervals) under each sounding string
        let bottom = top + sy * frets_shown as f32 + label_h * 0.55;
        for s in 0..6 {
            let Some(fret) = self.shape.frets[s] else {
                continue;
            };
            let pc = self.tuning.pitch(s, fret).pc();
            let text = match (self.style.label_mode, self.chord) {
                (LabelMode::Interval, Some(c)) => {
                    c.interval_of(pc).map(|i| i.label()).unwrap_or_default()
                }
                _ => self
                    .chord
                    .and_then(|c| c.notes().into_iter().find(|n| n.pc() == pc))
                    .unwrap_or_else(|| NoteName::from_pc(pc, theory::Spelling::Sharps))
                    .pretty(),
            };
            painter.text(
                Pos2::new(string_x(s), bottom),
                Align2::CENTER_CENTER,
                theme::glyphs(&text),
                FontId::proportional(sx * 0.4),
                theme::SOFT_INK,
            );
        }

        let clicked_string = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|p| {
                (0..6)
                    .min_by(|a, b| {
                        (string_x(*a) - p.x)
                            .abs()
                            .total_cmp(&(string_x(*b) - p.x).abs())
                    })
                    .filter(|s| (string_x(*s) - p.x).abs() < sx * 0.5)
            });
        ChordBoxResponse {
            response,
            clicked_string,
        }
    }
}

/// Plain-text description of a chord shape, for screen readers.
pub fn describe(shape: &ChordShape, name: &str) -> String {
    let finger_names = [
        "",
        "1st finger",
        "2nd finger",
        "3rd finger",
        "4th finger",
        "thumb",
    ];
    let mut parts = Vec::new();
    for s in 0..6 {
        let number = theory::string_number(s);
        match (shape.frets[s], shape.fingers[s]) {
            (None, _) => parts.push(format!("don't play string {number}")),
            (Some(0), _) => parts.push(format!("string {number} open")),
            (Some(f), Some(fi)) => parts.push(format!(
                "{} on string {number}, fret {f}",
                finger_names[fi.min(5) as usize]
            )),
            (Some(f), None) => parts.push(format!("string {number}, fret {f}")),
        }
    }
    format!("{name}: {}.", parts.join("; "))
}

/// A small, label-free chord diagram with the chord name above it (for cards and previews).
pub fn mini(ui: &mut Ui, shape: &ChordShape, name: &str, width: f32) -> Response {
    let sx = width / 6.2;
    let frets = 4u8;
    let sy = sx * 1.05;
    let top_pad = 46.0;
    let h = top_pad + sy * frets as f32 + 6.0;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, h), Sense::hover());
    let r = response.rect;
    // Leave room on the left for a two-digit fret number.
    let left = r.left() + sx * 0.85;
    let top = r.top() + top_pad;
    let base = if shape.fits_at_nut(4) {
        1
    } else {
        shape.lowest_fret().unwrap_or(1)
    };
    painter.text(
        Pos2::new(r.center().x, r.top() + 12.0),
        Align2::CENTER_CENTER,
        name,
        crate::theme::heading(20.0),
        theme::INK,
    );
    if base > 1 {
        // Which fret the top row is, for shapes up the neck.
        painter.text(
            Pos2::new(left - sx * 0.3, top + sy * 0.5),
            Align2::RIGHT_CENTER,
            format!("{base}"),
            egui::FontId::proportional(11.0),
            theme::SOFT_INK,
        );
    }
    let ink = Stroke::new(1.2, theme::INK);
    for s in 0..6 {
        let x = left + s as f32 * sx;
        painter.line_segment(
            [Pos2::new(x, top), Pos2::new(x, top + sy * frets as f32)],
            ink,
        );
    }
    for f in 0..=frets {
        let y = top + f as f32 * sy;
        painter.line_segment(
            [Pos2::new(left, y), Pos2::new(left + 5.0 * sx, y)],
            if f == 0 && base == 1 {
                Stroke::new(3.0, theme::INK)
            } else {
                ink
            },
        );
    }
    for s in 0..6 {
        let x = left + s as f32 * sx;
        match shape.frets[s] {
            None => {
                let c = Pos2::new(x, top - 11.0);
                let d = 4.0;
                painter.line_segment([c - Vec2::splat(d), c + Vec2::splat(d)], ink);
                painter.line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], ink);
            }
            Some(0) => {
                painter.circle_stroke(Pos2::new(x, top - 11.0), 4.5, ink);
            }
            Some(f) => {
                let row = (f + 1).saturating_sub(base) as f32 - 0.5;
                let color = shape.fingers[s]
                    .map(theme::finger_color)
                    .unwrap_or(theme::GRAY);
                painter.circle(
                    Pos2::new(x, top + row * sy),
                    sx * 0.36,
                    color,
                    Stroke::new(1.0, theme::INK.gamma_multiply(0.4)),
                );
            }
        }
    }
    super::describe::set(
        &response,
        super::describe::chord(shape, None, &Tuning::standard()),
    );
    response
}
