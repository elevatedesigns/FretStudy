//! A fretting hand with numbered, colored fingers, and a "six strings" picture.

use crate::theme;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

pub fn hand(ui: &mut Ui, width: f32) -> Response {
    let h = width * 0.75;
    let (response, painter) = ui.allocate_painter(Vec2::new(width, h), Sense::hover());
    super::describe::set(&response, "Fretting hand: finger 1 is the index (blue), 2 the middle (green), 3 the ring (orange), 4 the pinky (purple). The thumb stays behind the neck.".into());
    let r = response.rect;
    let palm = Rect::from_min_size(
        Pos2::new(r.left() + width * 0.3, r.top() + h * 0.48),
        Vec2::new(width * 0.56, h * 0.42),
    );
    let skin = Color32::from_rgb(0xF1, 0xC9, 0xA5);
    painter.rect_filled(palm, CornerRadius::same(30), skin);
    // Fingers: pointer → pinky, left to right
    let names = ["Pointer", "Middle", "Ring", "Pinky"];
    let lengths = [0.40, 0.46, 0.42, 0.32];
    let fw = palm.width() / 4.0;
    for i in 0..4 {
        let x = palm.left() + fw * (i as f32 + 0.5);
        let len = h * lengths[i];
        let finger = Rect::from_center_size(
            Pos2::new(x, palm.top() - len / 2.0 + 18.0),
            Vec2::new(fw * 0.78, len),
        );
        painter.rect_filled(finger, CornerRadius::same((fw * 0.39) as u8), skin);
        let tip = Pos2::new(x, finger.top() + fw * 0.4);
        let color = theme::finger_color(i as u8 + 1);
        painter.circle(
            tip,
            fw * 0.32,
            color,
            Stroke::new(1.5, theme::INK.gamma_multiply(0.5)),
        );
        painter.text(
            tip,
            Align2::CENTER_CENTER,
            (i + 1).to_string(),
            theme::heading(fw * 0.36),
            theme::text_on(color),
        );
        painter.text(
            Pos2::new(x, palm.center().y + 12.0),
            Align2::CENTER_CENTER,
            names[i],
            FontId::proportional(13.0),
            theme::INK,
        );
    }
    // Thumb
    let thumb_c = Pos2::new(palm.left() - width * 0.04, palm.center().y + 4.0);
    painter.circle_filled(thumb_c, fw * 0.42, skin);
    painter.circle(
        thumb_c,
        fw * 0.28,
        theme::GRAY,
        Stroke::new(1.5, theme::INK.gamma_multiply(0.5)),
    );
    painter.text(
        thumb_c,
        Align2::CENTER_CENTER,
        "T",
        theme::heading(fw * 0.3),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(r.left() + 2.0, thumb_c.y + fw * 0.62),
        Align2::LEFT_CENTER,
        "Thumb (behind the neck)",
        FontId::proportional(12.0),
        theme::SOFT_INK,
    );
    response
}

/// The six strings, thinnest on top (like tab and the neck map), with numbers and names.
pub fn strings(ui: &mut Ui, width: f32, highlight: Option<u8>) -> Response {
    let row = 30.0;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(width, row * 6.0 + 8.0), Sense::hover());
    super::describe::set(&response, "The six strings, thinnest to thickest: string 1 high E, string 2 B, string 3 G, string 4 D, string 5 A, string 6 low E.".into());
    let r = response.rect;
    let names = ["E", "B", "G", "D", "A", "E"];
    let notes = ["thinnest · high E", "", "", "", "", "thickest · low E"];
    for i in 0..6 {
        let number = i as u8 + 1;
        let y = r.top() + row * (i as f32 + 0.5);
        let lit = highlight == Some(number);
        let thick = 1.2 + i as f32 * 0.6;
        let color = if lit { theme::ORANGE } else { theme::STRING };
        painter.line_segment(
            [
                Pos2::new(r.left() + 120.0, y),
                Pos2::new(r.right() - 150.0, y),
            ],
            Stroke::new(thick + if lit { 2.0 } else { 0.0 }, color),
        );
        let badge = Pos2::new(r.left() + 40.0, y);
        painter.circle_filled(badge, 12.0, if lit { theme::ORANGE } else { theme::PANEL });
        painter.text(
            badge,
            Align2::CENTER_CENTER,
            number.to_string(),
            theme::heading(14.0),
            theme::INK,
        );
        painter.text(
            Pos2::new(r.left() + 84.0, y),
            Align2::CENTER_CENTER,
            names[i],
            theme::heading(20.0),
            theme::INK,
        );
        painter.text(
            Pos2::new(r.right() - 140.0, y),
            Align2::LEFT_CENTER,
            notes[i],
            FontId::proportional(13.0),
            theme::SOFT_INK,
        );
    }
    response
}
