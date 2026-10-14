//! Rhythm pictures: the strum grid (`1 & 2 & 3 & 4 &` with arrows) and the beat pulse.

use crate::theme;
use content::{Stroke as Strum, StrumPattern};
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

/// Draws an arrow pointing down (`down = true`) or up, centered in `rect`.
fn arrow(painter: &egui::Painter, rect: Rect, down: bool, color: Color32, ghost: bool) {
    let c = rect.center();
    let len = rect.height() * if down { 0.62 } else { 0.48 };
    let dir = if down { 1.0 } else { -1.0 };
    let start = Pos2::new(c.x, c.y - dir * len / 2.0);
    let tip = Pos2::new(c.x, c.y + dir * len / 2.0);
    let head = rect.width().min(rect.height()) * 0.22;
    let stroke = Stroke::new(if ghost { 2.0 } else { 4.0 }, color);
    if ghost {
        // Dashed shaft for a "ghost" strum: the hand moves but misses the strings.
        let segments = 5;
        for i in (0..segments).step_by(2) {
            let a = start.lerp(tip, i as f32 / segments as f32);
            let b = start.lerp(tip, (i + 1) as f32 / segments as f32);
            painter.line_segment([a, b], stroke);
        }
    } else {
        painter.line_segment([start, Pos2::new(tip.x, tip.y - dir * head * 0.6)], stroke);
    }
    let pts = vec![
        tip,
        Pos2::new(tip.x - head, tip.y - dir * head * 1.3),
        Pos2::new(tip.x + head, tip.y - dir * head * 1.3),
    ];
    if ghost {
        painter.add(Shape::closed_line(pts, Stroke::new(1.5, color)));
    } else {
        painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    }
}

/// The strum grid: one box per slot (`1 & 2 & 3 & 4 &`, or `1–6` in 6/8) with down/up/ghost
/// arrows, bass-note markers, or single-string picks. `playhead` highlights one slot.
pub fn strum_grid(
    ui: &mut Ui,
    pattern: StrumPattern,
    playhead: Option<usize>,
    width: f32,
) -> Response {
    let slots = pattern.slots();
    let counts = pattern.counts();
    let spb = pattern.slots_per_beat();
    let slot_w = (width / slots.len() as f32).min(70.0);
    let h = slot_w * 1.6;
    let (response, painter) = ui.allocate_painter(
        Vec2::new(slot_w * slots.len() as f32, h + 26.0),
        Sense::hover(),
    );
    super::describe::set(&response, super::describe::strum(pattern));
    let r = response.rect;
    for (i, stroke) in slots.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * slot_w, r.top()),
            Vec2::new(slot_w - 4.0, h),
        );
        let on_beat = i % spb == 0 && (spb == 1 || i % 2 == 0);
        let strong = if spb == 1 { i % 3 == 0 } else { on_beat };
        let lit = playhead == Some(i);
        let fill = if lit {
            theme::YELLOW.gamma_multiply(0.7)
        } else if strong {
            Color32::WHITE
        } else {
            theme::PANEL
        };
        painter.rect_filled(cell, CornerRadius::same(8), fill);
        painter.rect_stroke(
            cell,
            CornerRadius::same(8),
            Stroke::new(1.0, theme::WOOD_DARK),
            egui::StrokeKind::Inside,
        );
        let inner = cell.shrink(6.0);
        match stroke {
            Strum::Down => arrow(&painter, inner, true, theme::BLUE, false),
            Strum::Up => arrow(&painter, inner, false, theme::ORANGE, false),
            Strum::Ghost => arrow(&painter, inner, true, theme::LIGHT_GRAY, true),
            Strum::Bass(n) => {
                let c = inner.center();
                painter.circle(
                    c,
                    inner.width().min(inner.height()) * 0.36,
                    theme::VERMILLION,
                    Stroke::new(1.5, theme::INK.gamma_multiply(0.5)),
                );
                painter.text(
                    c,
                    Align2::CENTER_CENTER,
                    if *n == 0 { "p" } else { "p 2" },
                    FontId::proportional(slot_w * 0.2),
                    Color32::WHITE,
                );
            }
            Strum::Boogie(n) => {
                // Two stacked chips: the root plus the 5th or 6th above it.
                let c = inner.center();
                let rr = inner.width().min(inner.height()) * 0.2;
                painter.circle_filled(c + Vec2::new(0.0, rr * 1.1), rr, theme::VERMILLION);
                painter.circle_filled(
                    c - Vec2::new(0.0, rr * 1.1),
                    rr,
                    if *n == 0 { theme::GREEN } else { theme::PURPLE },
                );
                painter.text(
                    c - Vec2::new(0.0, rr * 1.1),
                    Align2::CENTER_CENTER,
                    if *n == 0 { "5" } else { "6" },
                    FontId::proportional(rr * 1.3),
                    Color32::WHITE,
                );
            }
            Strum::Pluck(n) => {
                let c = inner.center();
                painter.circle(
                    c,
                    inner.width().min(inner.height()) * 0.24,
                    theme::SKY_BLUE,
                    Stroke::new(1.5, theme::INK.gamma_multiply(0.5)),
                );
                painter.text(
                    c,
                    Align2::CENTER_CENTER,
                    // Fingerstyle letters: i plays the lowest of the three, a the highest.
                    // Tremolo plays one string with a, m, then i.
                    if pattern == StrumPattern::Tremolo {
                        ["p", "a", "m", "i"][i % 4]
                    } else {
                        match n {
                            2 => "i",
                            3 => "m",
                            _ => "a",
                        }
                    },
                    FontId::proportional(slot_w * 0.18),
                    theme::INK,
                );
            }
        }
        painter.text(
            Pos2::new(cell.center().x, r.top() + h + 13.0),
            Align2::CENTER_CENTER,
            counts[i],
            FontId::proportional(if strong { 18.0 } else { 15.0 }),
            if strong { theme::INK } else { theme::SOFT_INK },
        );
    }
    response
}

/// Beat circles (`beats` of them); `beat` lights up, `flash` (0..1) makes it pop on the click.
pub fn beat_pulse(
    ui: &mut Ui,
    beats: usize,
    beat: Option<usize>,
    flash: f32,
    width: f32,
) -> Response {
    let beats = beats.max(1);
    let cell = width / beats as f32;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(width, cell.min(90.0) * 0.9), Sense::hover());
    super::describe::set(
        &response,
        match beat {
            Some(b) => format!("Beat {} of {beats}", b + 1),
            None => format!("{beats} beats in a bar"),
        },
    );
    let r = response.rect;
    for i in 0..beats {
        let c = Pos2::new(r.left() + cell * (i as f32 + 0.5), r.center().y);
        let lit = beat == Some(i);
        let radius = cell.min(90.0) * 0.3 * if lit { 1.0 + 0.25 * flash } else { 1.0 };
        let fill = match (lit, i) {
            (true, 0) => theme::VERMILLION,
            (true, _) => theme::BLUE,
            _ => theme::PANEL,
        };
        painter.circle(c, radius, fill, Stroke::new(1.5, theme::WOOD_DARK));
        painter.text(
            c,
            Align2::CENTER_CENTER,
            (i + 1).to_string(),
            theme::heading(cell.min(90.0) * 0.28),
            if lit { Color32::WHITE } else { theme::SOFT_INK },
        );
    }
    response
}
