//! Guitar tab: six lines (thinnest string on top), fret numbers in colored chips, and
//! spacing that shows how long each note lasts.

use crate::theme;
use content::TabNote;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// Which finger usually plays a fret in first position (for chip colors).
fn first_position_finger(fret: u8) -> Option<u8> {
    match fret {
        1..=4 => Some(fret),
        _ => None,
    }
}

pub struct TabStaff<'a> {
    pub notes: &'a [TabNote],
    /// Note index to highlight (playhead or "play this next").
    pub current: Option<usize>,
    /// Notes already played correctly (drawn with a check).
    pub done: usize,
    pub width: f32,
    /// Room before the first note (matches the staff above when both are drawn).
    pub lead: f32,
    /// Beats in a bar (bar lines and counts).
    pub bar: f32,
}

impl<'a> TabStaff<'a> {
    pub fn show(self, ui: &mut Ui) -> Response {
        let line_gap = 18.0;
        let top_pad = 30.0;
        let h = top_pad + line_gap * 5.0 + 50.0;
        let (response, painter) = ui.allocate_painter(Vec2::new(self.width, h), Sense::hover());
        crate::widgets::describe::set(&response, crate::widgets::describe::tab(self.notes));
        let r = response.rect;
        let left = r.left() + self.lead - 10.0;
        let right = r.right() - 10.0;
        let y_for = |string: u8| r.top() + top_pad + (string as f32 - 1.0) * line_gap;

        // "TAB" clef and string letters
        let letters = ["e", "B", "G", "D", "A", "E"];
        for (i, l) in letters.iter().enumerate() {
            let y = y_for(i as u8 + 1);
            painter.line_segment(
                [Pos2::new(left, y), Pos2::new(right, y)],
                Stroke::new(1.2, theme::SOFT_INK),
            );
            painter.text(
                Pos2::new(r.left() + 14.0, y),
                Align2::CENTER_CENTER,
                *l,
                FontId::monospace(13.0),
                theme::SOFT_INK,
            );
        }
        painter.line_segment(
            [Pos2::new(left, y_for(1)), Pos2::new(left, y_for(6))],
            Stroke::new(2.0, theme::INK),
        );
        painter.line_segment(
            [Pos2::new(right, y_for(1)), Pos2::new(right, y_for(6))],
            Stroke::new(2.0, theme::INK),
        );

        // Spacing proportional to note length, so long notes look long.
        // Short notes still get room to read; longer notes get proportionally more.
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
        let mut prev_x = left;
        for (i, n) in self.notes.iter().enumerate() {
            // Bar line at the start of every new measure (every 4 beats).
            if i > 0 && beat_pos > 0.0 && (beat_pos % self.bar).abs() < 1e-3 {
                let bx = x - usable * 0.25 / total * 2.0;
                painter.line_segment(
                    [Pos2::new(bx, y_for(1)), Pos2::new(bx, y_for(6))],
                    Stroke::new(1.2, theme::SOFT_INK),
                );
            }
            let y = y_for(n.string.clamp(1, 6));
            if let Some(tech) = n.tech {
                // Bends and vibrato mark the note itself; the rest sit between the two notes.
                let (pos, text) = match tech {
                    content::Tech::Bend(1) => (Pos2::new(x + 14.0, y - 13.0), "b½".to_string()),
                    content::Tech::Bend(_) => (Pos2::new(x + 14.0, y - 13.0), "b full".to_string()),
                    content::Tech::Vibrato => (Pos2::new(x + 14.0, y - 13.0), "~~".to_string()),
                    other => {
                        // A slur arc from the previous note, like printed tab.
                        let arc = egui::epaint::QuadraticBezierShape::from_points_stroke(
                            [
                                Pos2::new(prev_x + 6.0, y - 10.0),
                                Pos2::new((prev_x + x) / 2.0, y - 22.0),
                                Pos2::new(x - 6.0, y - 10.0),
                            ],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(1.5, theme::VERMILLION),
                        );
                        painter.add(arc);
                        (
                            Pos2::new((prev_x + x) / 2.0, y - 24.0),
                            other.mark().to_string(),
                        )
                    }
                };
                let align = if matches!(
                    tech,
                    content::Tech::Hammer | content::Tech::Pull | content::Tech::Slide
                ) {
                    Align2::CENTER_BOTTOM
                } else {
                    Align2::LEFT_CENTER
                };
                painter.text(
                    pos,
                    align,
                    text,
                    FontId::proportional(13.0),
                    theme::VERMILLION,
                );
            }
            prev_x = x;
            let label = n.fret.to_string();
            let chip_w = if n.fret >= 10 { 26.0 } else { 20.0 };
            let chip = Rect::from_center_size(Pos2::new(x, y), Vec2::new(chip_w, 18.0));
            let fill = first_position_finger(n.fret)
                .map(theme::finger_color)
                .unwrap_or(Color32::WHITE);
            let is_current = self.current == Some(i);
            if is_current {
                painter.circle_filled(Pos2::new(x, y), 17.0, theme::YELLOW.gamma_multiply(0.8));
            }
            painter.rect_filled(chip, CornerRadius::same(6), fill);
            painter.rect_stroke(
                chip,
                CornerRadius::same(6),
                Stroke::new(1.2, theme::INK.gamma_multiply(0.6)),
                egui::StrokeKind::Middle,
            );
            painter.text(
                chip.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(14.0),
                theme::text_on(fill),
            );
            if i < self.done {
                painter.text(
                    Pos2::new(x, y_for(6) + 18.0),
                    Align2::CENTER_CENTER,
                    "✔",
                    FontId::proportional(14.0),
                    theme::GREEN,
                );
            }
            // Count under the staff: show the beat number when the note starts on a beat.
            if (beat_pos.fract()).abs() < 1e-3 {
                painter.text(
                    Pos2::new(x, y_for(6) + 36.0),
                    Align2::CENTER_CENTER,
                    format!("{}", (beat_pos as usize % self.bar as usize) + 1),
                    FontId::proportional(13.0),
                    theme::SOFT_INK,
                );
            } else if (beat_pos.fract() - 0.5).abs() < 1e-3 {
                painter.text(
                    Pos2::new(x, y_for(6) + 36.0),
                    Align2::CENTER_CENTER,
                    "&",
                    FontId::proportional(12.0),
                    theme::SOFT_INK,
                );
            }
            beat_pos += n.beats;
            x += usable * space(n.beats) / total;
        }
        response
    }
}

/// A fixed-width tab that wraps long melodies onto rows of `per_row` notes, each with
/// standard notation above it when `staff` gives a tuning (and key).
#[allow(clippy::too_many_arguments)]
pub fn notation_rows(
    ui: &mut Ui,
    notes: &[TabNote],
    current: Option<usize>,
    done: usize,
    width: f32,
    per_row: usize,
    staff: Option<(&theory::Tuning, Option<&theory::Key>)>,
    bar: u8,
) {
    let bar = bar.max(1) as f32;
    let lead = crate::widgets::staff::lead(staff.and_then(|s| s.1));
    for (row, chunk) in notes.chunks(per_row).enumerate() {
        let start = row * per_row;
        let row_current = current
            .and_then(|c| c.checked_sub(start))
            .filter(|c| *c < chunk.len());
        if let Some((tuning, key)) = staff {
            crate::widgets::staff::StaffNotation {
                notes: chunk,
                tuning,
                key,
                current: row_current,
                width,
                bar,
            }
            .show(ui);
        }
        TabStaff {
            lead,
            bar,
            notes: chunk,
            current: current
                .and_then(|c| c.checked_sub(start))
                .filter(|c| *c < chunk.len()),
            done: done.saturating_sub(start).min(chunk.len()),
            width,
        }
        .show(ui);
    }
}
