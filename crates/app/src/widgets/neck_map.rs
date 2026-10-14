//! The horizontal neck map: headstock on the left, thinnest string (high E) on top so it
//! lines up with tab. Mirrored for left-handed players.

use super::{Dot, paint_dot};
use crate::theme;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use theory::{NoteName, Spelling, Tuning};

pub struct NeckMap<'a> {
    pub tuning: &'a Tuning,
    pub dots: &'a [Dot],
    /// First fret drawn (0 shows the nut and open strings).
    pub first_fret: u8,
    pub last_fret: u8,
    pub left_handed: bool,
    pub height: f32,
    /// Per-string glow (0..1) while ringing.
    pub glow: [f32; 6],
    /// A position to highlight with a ring (e.g. the note just played).
    pub highlight: Option<(usize, u8)>,
    /// Draw a capo just behind this fret.
    pub capo: Option<u8>,
    /// A light-brown maple board with grain and dark inlays (Neck Explorer).
    pub maple: bool,
}

pub struct NeckMapResponse {
    /// Any position the learner clicked, marked or not.
    pub clicked: Option<(usize, u8)>,
}

impl<'a> NeckMap<'a> {
    pub fn new(tuning: &'a Tuning, dots: &'a [Dot]) -> Self {
        NeckMap {
            tuning,
            dots,
            first_fret: 0,
            last_fret: 12,
            left_handed: false,
            height: 230.0,
            glow: [0.0; 6],
            highlight: None,
            capo: None,
            maple: false,
        }
    }

    pub fn frets(mut self, first: u8, last: u8) -> Self {
        self.first_fret = first;
        self.last_fret = last.max(first + 1);
        self
    }

    pub fn left_handed(mut self, yes: bool) -> Self {
        self.left_handed = yes;
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn glow(mut self, glow: [f32; 6]) -> Self {
        self.glow = glow;
        self
    }

    pub fn maple(mut self, yes: bool) -> Self {
        self.maple = yes;
        self
    }

    pub fn capo(mut self, fret: Option<u8>) -> Self {
        self.capo = fret;
        self
    }

    pub fn highlight(mut self, pos: Option<(usize, u8)>) -> Self {
        self.highlight = pos;
        self
    }

    pub fn show(self, ui: &mut Ui) -> NeckMapResponse {
        let w = ui.available_width().max(300.0);
        let (response, painter) = ui.allocate_painter(Vec2::new(w, self.height), Sense::click());
        super::describe::set(
            &response,
            super::describe::neck(self.dots, self.first_fret, self.last_fret),
        );
        let rect = response.rect;

        let label_w = 34.0; // string names
        let open_w = if self.first_fret == 0 { 40.0 } else { 0.0 };
        let numbers_h = 26.0;
        let pad_top = 8.0;
        let board_top = rect.top() + pad_top;
        let board_h = self.height - numbers_h - pad_top;
        let row = board_h / 6.0;
        let first_fretted = self.first_fret.max(1);
        let spaces = (self.last_fret - first_fretted + 1) as f32;
        let board_left = rect.left() + label_w + open_w;
        let board_right = rect.right() - 8.0;
        let fw = (board_right - board_left) / spaces;

        // Mirroring for left-handed players flips the x axis around the board.
        let flip = |x: f32| {
            if self.left_handed {
                rect.left() + rect.right() - x
            } else {
                x
            }
        };
        let string_y = |s: usize| board_top + (5 - s) as f32 * row + row * 0.5; // thinnest on top
        let fret_center_x = |f: u8| {
            if f == 0 {
                flip(rect.left() + label_w + open_w * 0.5)
            } else {
                flip(board_left + (f - first_fretted) as f32 * fw + fw * 0.5)
            }
        };
        let wire_x = |i: f32| flip(board_left + i * fw);

        // Wood
        let wood = Rect::from_x_y_ranges(
            flip(board_left).min(flip(board_right))..=flip(board_left).max(flip(board_right)),
            board_top..=board_top + board_h,
        );
        let (wood_c, inlay, fret_c, string_c, nut_c) = if self.maple {
            (
                egui::Color32::from_rgb(0xE6, 0xC3, 0x8F),
                egui::Color32::from_rgb(0x4A, 0x3A, 0x2C),
                egui::Color32::from_rgb(0xA8, 0xA1, 0x97),
                egui::Color32::from_rgb(0x6A, 0x60, 0x55),
                egui::Color32::from_rgb(0xF4, 0xEC, 0xDC),
            )
        } else {
            (
                theme::WOOD,
                theme::WOOD_DARK,
                theme::FRET,
                theme::STRING,
                theme::NUT,
            )
        };
        painter.rect_filled(wood, CornerRadius::same(6), wood_c);
        if self.maple {
            // Soft, wavy maple grain.
            for k in 0..14 {
                let y0 = board_top + board_h * (k as f32 + 0.5) / 14.0;
                let pts: Vec<Pos2> = (0..=40)
                    .map(|i| {
                        let t = i as f32 / 40.0;
                        let x = wood.left() + t * wood.width();
                        Pos2::new(x, y0 + (t * 9.0 + k as f32 * 1.7).sin() * 1.6)
                    })
                    .collect();
                painter.add(egui::Shape::line(
                    pts,
                    Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(0x9A, 0x6A, 0x30, 30),
                    ),
                ));
            }
        }

        // Inlay markers
        for f in first_fretted..=self.last_fret {
            let x = fret_center_x(f);
            let mid = board_top + board_h * 0.5;
            match f % 12 {
                3 | 5 | 7 | 9 => {
                    painter.circle_filled(Pos2::new(x, mid), row * 0.16, inlay);
                }
                0 => {
                    painter.circle_filled(Pos2::new(x, mid - row * 1.5), row * 0.16, inlay);
                    painter.circle_filled(Pos2::new(x, mid + row * 1.5), row * 0.16, inlay);
                }
                _ => {}
            }
        }
        // Fret wires (and the nut)
        for i in 0..=(spaces as u32) {
            let x = wire_x(i as f32);
            let is_nut = i == 0 && self.first_fret <= 1;
            let (width, color) = if is_nut { (6.0, nut_c) } else { (2.0, fret_c) };
            painter.line_segment(
                [Pos2::new(x, board_top), Pos2::new(x, board_top + board_h)],
                Stroke::new(width, color),
            );
        }
        // Strings + names
        let names = self.tuning.open_names();
        for s in 0..6 {
            let y = string_y(s);
            let glow = self.glow[s];
            let thickness = 1.0 + (5 - s) as f32 * 0.4 + glow * 1.5;
            let color = if glow > 0.0 {
                string_c.lerp_to_gamma(theme::ORANGE, glow)
            } else {
                string_c
            };
            let x0 = if self.first_fret == 0 {
                flip(rect.left() + label_w + 6.0)
            } else {
                flip(board_left)
            };
            painter.line_segment(
                [Pos2::new(x0, y), Pos2::new(flip(board_right), y)],
                Stroke::new(thickness, color),
            );
            let name_x = flip(rect.left() + label_w * 0.5);
            painter.text(
                Pos2::new(name_x, y),
                Align2::CENTER_CENTER,
                names[s].to_string(),
                FontId::proportional(row * 0.42),
                theme::SOFT_INK,
            );
        }
        // Fret numbers
        for f in first_fretted..=self.last_fret {
            let strong = matches!(f % 12, 0 | 3 | 5 | 7 | 9);
            painter.text(
                Pos2::new(fret_center_x(f), board_top + board_h + numbers_h * 0.55),
                Align2::CENTER_CENTER,
                f.to_string(),
                FontId::proportional(if strong { 15.0 } else { 12.0 }),
                if strong { theme::INK } else { theme::SOFT_INK },
            );
        }

        // Capo: a bar across all strings, just behind the fret wire.
        if let Some(c) = self.capo
            && (first_fretted..=self.last_fret).contains(&c)
        {
            let wire = wire_x((c - first_fretted + 1) as f32);
            let x = if self.left_handed {
                wire + fw * 0.18
            } else {
                wire - fw * 0.18
            };
            let bar = Rect::from_center_size(
                Pos2::new(x, board_top + board_h / 2.0),
                Vec2::new(fw * 0.22, board_h + 10.0),
            );
            painter.rect_filled(bar, CornerRadius::same(5), theme::INK.gamma_multiply(0.85));
            for (i, ch) in "CAPO".chars().enumerate() {
                painter.text(
                    Pos2::new(x, board_top + 12.0 + i as f32 * 12.0),
                    Align2::CENTER_CENTER,
                    ch,
                    FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }
        }
        // Highlight ring
        if let Some((s, f)) = self.highlight
            && (f == 0 || (first_fretted..=self.last_fret).contains(&f))
        {
            painter.circle_stroke(
                Pos2::new(fret_center_x(f), string_y(s)),
                row * 0.5,
                Stroke::new(3.0, theme::ORANGE),
            );
        }
        // Dots
        let radius = (row * 0.38).min(fw * 0.4);
        for dot in self.dots {
            let visible = dot.fret == 0 && self.first_fret == 0
                || (first_fretted..=self.last_fret).contains(&dot.fret) && dot.fret > 0;
            if visible {
                paint_dot(
                    &painter,
                    Pos2::new(fret_center_x(dot.fret), string_y(dot.string)),
                    radius,
                    dot,
                    0.0,
                );
            }
        }

        // Hit testing: which (string, fret) is under a point?
        let locate = |p: Pos2| -> Option<(usize, u8)> {
            if !rect.contains(p) {
                return None;
            }
            let s_row = ((p.y - board_top) / row).floor();
            if !(0.0..6.0).contains(&s_row) {
                return None;
            }
            let string = 5 - s_row as usize;
            let x = flip(p.x); // back to right-handed coordinates
            let fret = if x < board_left {
                if self.first_fret == 0 && x > rect.left() + label_w {
                    0
                } else {
                    return None;
                }
            } else {
                let i = ((x - board_left) / fw).floor() as u8;
                (first_fretted + i).min(self.last_fret)
            };
            Some((string, fret))
        };
        let clicked = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(locate);
        let hovered = response.hover_pos().and_then(locate);
        if let Some((s, f)) = hovered {
            let name = NoteName::from_pc(self.tuning.pitch(s, f).pc(), Spelling::Sharps);
            response.clone().on_hover_text(format!(
                "String {}, fret {f}: {}",
                theory::string_number(s),
                theme::glyphs(&name.pretty())
            ));
        }
        NeckMapResponse { clicked }
    }
}
