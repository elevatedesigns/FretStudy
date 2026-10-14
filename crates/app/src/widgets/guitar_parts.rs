//! An acoustic guitar drawn from shapes, with clickable parts (Lesson L0.1).

use crate::theme;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Headstock,
    TuningPegs,
    Nut,
    Neck,
    Frets,
    FretMarkers,
    Body,
    Soundhole,
    Bridge,
    Strings,
}

impl Part {
    pub const ALL: [Part; 10] = [
        Part::Headstock,
        Part::TuningPegs,
        Part::Nut,
        Part::Neck,
        Part::Frets,
        Part::FretMarkers,
        Part::Body,
        Part::Soundhole,
        Part::Bridge,
        Part::Strings,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Part::Headstock => "Headstock",
            Part::TuningPegs => "Tuning pegs",
            Part::Nut => "Nut",
            Part::Neck => "Neck",
            Part::Frets => "Frets",
            Part::FretMarkers => "Fret markers",
            Part::Body => "Body",
            Part::Soundhole => "Soundhole",
            Part::Bridge => "Bridge",
            Part::Strings => "Strings",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Part::Headstock => "The top of the guitar, where the tuning pegs live.",
            Part::TuningPegs => "Turn these to tighten (higher) or loosen (lower) each string.",
            Part::Nut => "A little bar that holds the strings up at the top of the neck.",
            Part::Neck => "The long part your fretting hand holds.",
            Part::Frets => "Metal bars across the neck. Each one is one step higher.",
            Part::FretMarkers => "Dots on frets 3, 5, 7, 9, and 12 that help you find your place.",
            Part::Body => "The big wooden box. It makes the sound louder.",
            Part::Soundhole => "The round hole where the sound comes out.",
            Part::Bridge => {
                "Holds the strings at the body end. The white strip on it is the saddle."
            }
            Part::Strings => "Six strings. When one shakes, it makes a sound.",
        }
    }
}

pub struct GuitarParts {
    pub width: f32,
    /// A part to highlight (hovered, the answer, or the one being asked about).
    pub highlight: Option<Part>,
    pub highlight_color: Color32,
}

pub struct GuitarPartsResponse {
    pub clicked: Option<Part>,
    pub hovered: Option<Part>,
}

struct Layout {
    rect: Rect,
    head: Rect,
    board: Rect,
    nut_x: f32,
    fret_xs: Vec<f32>,
    markers: Vec<Pos2>,
    upper: (Pos2, f32),
    lower: (Pos2, f32),
    hole: (Pos2, f32),
    bridge: Rect,
    string_ys: Vec<f32>,
    pegs: Vec<Pos2>,
}

impl Layout {
    fn new(rect: Rect) -> Layout {
        let w = rect.width();
        let h = rect.height();
        let cy = rect.center().y;
        let x = |f: f32| rect.left() + f * w;
        let board_h = h * 0.17;
        let head = Rect::from_min_max(
            Pos2::new(x(0.02), cy - h * 0.15),
            Pos2::new(x(0.13), cy + h * 0.15),
        );
        let nut_x = x(0.135);
        let board = Rect::from_min_max(
            Pos2::new(nut_x, cy - board_h / 2.0),
            Pos2::new(x(0.60), cy + board_h / 2.0),
        );
        // Frets get closer together toward the body, like a real guitar.
        let scale = (x(0.86) - nut_x) * 1.6;
        let fret_xs: Vec<f32> = (1..=12)
            .map(|n| nut_x + scale * (1.0 - 2f32.powf(-(n as f32) / 12.0)))
            .filter(|fx| *fx < board.right() - 2.0)
            .collect();
        let center_of = |n: usize| {
            let a = if n == 1 { nut_x } else { fret_xs[n - 2] };
            (a + fret_xs[n - 1]) / 2.0
        };
        let mut markers = Vec::new();
        for n in [3usize, 5, 7, 9] {
            if n <= fret_xs.len() {
                markers.push(Pos2::new(center_of(n), cy));
            }
        }
        if fret_xs.len() >= 12 {
            markers.push(Pos2::new(center_of(12), cy - board_h * 0.25));
            markers.push(Pos2::new(center_of(12), cy + board_h * 0.25));
        }
        let upper = (Pos2::new(x(0.64), cy), h * 0.36);
        let lower = (Pos2::new(x(0.82), cy), h * 0.46);
        let hole = (Pos2::new(x(0.665), cy), h * 0.13);
        let bridge = Rect::from_center_size(Pos2::new(x(0.86), cy), Vec2::new(w * 0.028, h * 0.34));
        let string_ys: Vec<f32> = (0..6)
            .map(|i| cy - board_h * 0.38 + i as f32 * board_h * 0.76 / 5.0)
            .collect();
        let pegs: Vec<Pos2> = (0..3)
            .flat_map(|i| {
                let px = head.left() + head.width() * (0.25 + 0.27 * i as f32);
                [
                    Pos2::new(px, head.top() - 6.0),
                    Pos2::new(px, head.bottom() + 6.0),
                ]
            })
            .collect();
        Layout {
            rect,
            head,
            board,
            nut_x,
            fret_xs,
            markers,
            upper,
            lower,
            hole,
            bridge,
            string_ys,
            pegs,
        }
    }

    fn hit(&self, p: Pos2) -> Option<Part> {
        if !self.rect.contains(p) {
            return None;
        }
        if self.pegs.iter().any(|g| g.distance(p) < 12.0) {
            return Some(Part::TuningPegs);
        }
        if (p.x - self.nut_x).abs() < 5.0 && (self.board.top()..=self.board.bottom()).contains(&p.y)
        {
            return Some(Part::Nut);
        }
        let on_strings_x = p.x > self.nut_x && p.x < self.bridge.center().x;
        if on_strings_x && self.string_ys.iter().any(|y| (p.y - y).abs() < 3.0) {
            return Some(Part::Strings);
        }
        if self.board.contains(p) {
            if self.markers.iter().any(|m| m.distance(p) < 8.0) {
                return Some(Part::FretMarkers);
            }
            if self.fret_xs.iter().any(|fx| (p.x - fx).abs() < 4.0) {
                return Some(Part::Frets);
            }
            return Some(Part::Neck);
        }
        if self.head.expand(4.0).contains(p) {
            return Some(Part::Headstock);
        }
        if self.hole.0.distance(p) < self.hole.1 {
            return Some(Part::Soundhole);
        }
        if self.bridge.expand(4.0).contains(p) {
            return Some(Part::Bridge);
        }
        let in_body = self.upper.0.distance(p) < self.upper.1
            || self.lower.0.distance(p) < self.lower.1
            || (p.x > self.upper.0.x
                && p.x < self.lower.0.x
                && (p.y - self.upper.0.y).abs() < self.upper.1 * 0.8);
        if in_body {
            return Some(Part::Body);
        }
        None
    }
}

impl GuitarParts {
    pub fn new(width: f32) -> Self {
        GuitarParts {
            width,
            highlight: None,
            highlight_color: theme::ORANGE,
        }
    }

    pub fn show(self, ui: &mut Ui) -> GuitarPartsResponse {
        let size = Vec2::new(self.width, self.width * 0.36);
        let (response, painter) = ui.allocate_painter(size, Sense::click());
        let l = Layout::new(response.rect);
        let hovered = response.hover_pos().and_then(|p| l.hit(p));
        let lit = self.highlight.or(hovered);
        let glow = |part: Part| lit == Some(part);
        let hl = self.highlight_color;
        let edge = Stroke::new(1.5, theme::NUT.gamma_multiply(0.6));

        // Body (two bouts and a waist)
        let body_fill = if glow(Part::Body) {
            hl.gamma_multiply(0.6)
        } else {
            Color32::from_rgb(0xD9, 0xA8, 0x6C)
        };
        // Outline first (slightly larger, in the edge color), then the fill on top, so the
        // two bouts merge into one smooth body without inner lines.
        let waist = Rect::from_min_max(
            Pos2::new(l.upper.0.x, l.upper.0.y - l.upper.1 * 0.8),
            Pos2::new(l.lower.0.x, l.lower.0.y + l.upper.1 * 0.8),
        );
        painter.circle_filled(l.upper.0, l.upper.1 + 1.5, edge.color);
        painter.circle_filled(l.lower.0, l.lower.1 + 1.5, edge.color);
        painter.circle_filled(l.upper.0, l.upper.1, body_fill);
        painter.circle_filled(l.lower.0, l.lower.1, body_fill);
        painter.rect_filled(waist, CornerRadius::ZERO, body_fill);
        // Soundhole with rosette
        painter.circle_filled(
            l.hole.0,
            l.hole.1 + 5.0,
            Color32::from_rgb(0x8A, 0x5A, 0x2E),
        );
        painter.circle_filled(
            l.hole.0,
            l.hole.1,
            if glow(Part::Soundhole) {
                hl
            } else {
                Color32::from_rgb(0x2A, 0x1C, 0x10)
            },
        );
        // Bridge + saddle
        painter.rect_filled(
            l.bridge,
            CornerRadius::same(4),
            if glow(Part::Bridge) {
                hl
            } else {
                Color32::from_rgb(0x3B, 0x26, 0x18)
            },
        );
        painter.line_segment(
            [
                Pos2::new(l.bridge.center().x - 2.0, l.bridge.top() + 4.0),
                Pos2::new(l.bridge.center().x - 2.0, l.bridge.bottom() - 4.0),
            ],
            Stroke::new(2.5, Color32::from_rgb(0xF4, 0xEE, 0xDF)),
        );
        // Headstock
        painter.rect(
            l.head,
            CornerRadius::same(10),
            if glow(Part::Headstock) {
                hl
            } else {
                Color32::from_rgb(0x4A, 0x31, 0x1F)
            },
            edge,
            egui::StrokeKind::Middle,
        );
        for g in &l.pegs {
            painter.circle(
                *g,
                7.0,
                if glow(Part::TuningPegs) {
                    hl
                } else {
                    Color32::from_rgb(0xC8, 0xC2, 0xB6)
                },
                edge,
            );
        }
        // Neck / fretboard
        painter.rect_filled(
            l.board,
            CornerRadius::same(2),
            if glow(Part::Neck) {
                hl
            } else {
                Color32::from_rgb(0x55, 0x3A, 0x27)
            },
        );
        for fx in &l.fret_xs {
            painter.line_segment(
                [
                    Pos2::new(*fx, l.board.top()),
                    Pos2::new(*fx, l.board.bottom()),
                ],
                Stroke::new(
                    if glow(Part::Frets) { 4.0 } else { 2.0 },
                    if glow(Part::Frets) {
                        hl
                    } else {
                        Color32::from_rgb(0xC9, 0xC6, 0xBF)
                    },
                ),
            );
        }
        for m in &l.markers {
            painter.circle_filled(
                *m,
                4.5,
                if glow(Part::FretMarkers) {
                    hl
                } else {
                    Color32::from_rgb(0xEE, 0xE7, 0xD6)
                },
            );
        }
        painter.line_segment(
            [
                Pos2::new(l.nut_x, l.board.top() - 2.0),
                Pos2::new(l.nut_x, l.board.bottom() + 2.0),
            ],
            Stroke::new(
                if glow(Part::Nut) { 7.0 } else { 5.0 },
                if glow(Part::Nut) {
                    hl
                } else {
                    Color32::from_rgb(0xF4, 0xEE, 0xDF)
                },
            ),
        );
        // Strings: from the pegs over the nut to the bridge
        for (i, y) in l.string_ys.iter().enumerate() {
            let thick = 1.0 + (5 - i.min(5)) as f32 * 0.25;
            let color = if glow(Part::Strings) {
                hl
            } else {
                Color32::from_rgb(0xDD, 0xD6, 0xC8)
            };
            painter.line_segment(
                [Pos2::new(l.nut_x, *y), Pos2::new(l.bridge.center().x, *y)],
                Stroke::new(thick + if glow(Part::Strings) { 1.0 } else { 0.0 }, color),
            );
            let peg = l.pegs[(i % 3) * 2 + if i < 3 { 0 } else { 1 }];
            painter.line_segment(
                [
                    Pos2::new(l.nut_x, *y),
                    Pos2::new(peg.x, l.head.center().y + (y - l.head.center().y) * 0.6),
                ],
                Stroke::new(1.0, color.gamma_multiply(0.7)),
            );
        }

        if let Some(p) = lit {
            let pos = Pos2::new(l.rect.center().x, l.rect.bottom() - 4.0);
            painter.text(
                pos,
                Align2::CENTER_BOTTOM,
                p.name(),
                theme::heading(18.0),
                theme::INK,
            );
        }
        let clicked = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|p| l.hit(p));
        GuitarPartsResponse { clicked, hovered }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_can_be_clicked() {
        let l = Layout::new(Rect::from_min_size(Pos2::ZERO, Vec2::new(720.0, 260.0)));
        let mut found = std::collections::HashSet::new();
        for x in (0..720).step_by(2) {
            for y in (0..260).step_by(2) {
                if let Some(p) = l.hit(Pos2::new(x as f32 + 0.5, y as f32 + 0.5)) {
                    found.insert(p);
                }
            }
        }
        for p in Part::ALL {
            assert!(found.contains(&p), "{p:?} has no clickable area");
        }
    }
}
