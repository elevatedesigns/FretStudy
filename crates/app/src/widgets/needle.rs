//! The tuner needle: a half-circle gauge from −50 to +50 cents.

use crate::theme;
use egui::{Align2, FontId, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};

pub struct TunerNeedle {
    /// Cents off (None = no note heard).
    pub cents: Option<f32>,
    /// Big text in the middle (the note or string name).
    pub label: String,
    pub sublabel: String,
    pub size: f32,
    /// False while showing a held reading after the note faded (drawn dimmer).
    pub live: bool,
}

impl TunerNeedle {
    pub fn show(self, ui: &mut Ui) -> Response {
        let w = self.size;
        let h = w * 0.92 + 30.0;
        let (response, painter) = ui.allocate_painter(Vec2::new(w, h), Sense::hover());
        let reading = match self.cents {
            Some(c) if c.abs() <= 5.0 => format!("{}: in tune", self.label),
            Some(c) if c < 0.0 => {
                format!("{}: {:.0} cents flat, tighten the string", self.label, -c)
            }
            Some(c) => format!("{}: {:.0} cents sharp, loosen the string", self.label, c),
            None => "Tuner: no note heard".to_string(),
        };
        super::describe::set(&response, reading);
        let rect = response.rect;
        let p = &painter;
        let cx = rect.center().x;
        // Note name and frequency on top.
        p.text(
            Pos2::new(cx, rect.top() + w * 0.10),
            Align2::CENTER_CENTER,
            &self.label,
            theme::heading(w * 0.17),
            theme::INK,
        );
        p.text(
            Pos2::new(cx, rect.top() + w * 0.215),
            Align2::CENTER_CENTER,
            &self.sublabel,
            FontId::proportional(18.0),
            theme::SOFT_INK,
        );
        // The gauge: a thick half-ring from −50 to +50 cents.
        let center = Pos2::new(cx, rect.top() + w * 0.75);
        let radius = w * 0.40;
        let band = w * 0.065;
        let angle_for =
            |cents: f32| std::f32::consts::PI * (0.5 + cents.clamp(-50.0, 50.0) / 100.0);
        let point = |cents: f32, r: f32| {
            let a = angle_for(cents);
            Pos2::new(center.x - r * a.cos(), center.y - r * a.sin())
        };
        let zone = |from: f32, to: f32, color: egui::Color32| {
            let steps = 30;
            let pts: Vec<Pos2> = (0..=steps)
                .map(|i| point(from + (to - from) * i as f32 / steps as f32, radius))
                .collect();
            p.add(Shape::line(pts, Stroke::new(band, color)));
        };
        let red = egui::Color32::from_rgb(0xF2, 0x76, 0x3A);
        let yellow = egui::Color32::from_rgb(0xF4, 0xC9, 0x4A);
        let green = egui::Color32::from_rgb(0x1E, 0xA0, 0x6A);
        zone(-50.0, -16.0, red);
        zone(-16.0, -6.0, yellow);
        zone(-6.0, 6.0, green);
        zone(6.0, 16.0, yellow);
        zone(16.0, 50.0, red);
        for c in (-50..=50).step_by(10) {
            let inner = point(c as f32, radius - band * 0.5 - w * 0.06);
            let outer = point(c as f32, radius - band * 0.5 - w * 0.015);
            p.line_segment(
                [inner, outer],
                Stroke::new(if c == 0 { 3.0 } else { 1.8 }, theme::SOFT_INK),
            );
        }
        let (flat, sharp) = if theme::has_music_glyphs() {
            ("♭ Low", "High ♯")
        } else {
            ("Low", "High")
        };
        let below = center.y + 22.0;
        p.text(
            Pos2::new(point(-50.0, radius).x - band * 0.5, below),
            Align2::LEFT_CENTER,
            flat,
            FontId::proportional(17.0),
            theme::SOFT_INK,
        );
        p.text(
            Pos2::new(point(50.0, radius).x + band * 0.5, below),
            Align2::RIGHT_CENTER,
            sharp,
            FontId::proportional(17.0),
            theme::SOFT_INK,
        );
        // Needle and hub.
        let needle_color = if self.live {
            theme::INK
        } else {
            theme::INK.gamma_multiply(0.35)
        };
        if let Some(c) = self.cents {
            p.line_segment(
                [center, point(c, radius - band * 0.6)],
                Stroke::new(w * 0.012, needle_color),
            );
        }
        p.circle_filled(center, w * 0.045, egui::Color32::from_rgb(0xEC, 0xE6, 0xDE));
        p.circle_filled(
            center,
            w * 0.028,
            if self.cents.is_some() {
                needle_color
            } else {
                theme::LIGHT_GRAY
            },
        );
        // Status box under the gauge.
        let (fill, ink, title, line) = match self.cents {
            Some(_) if !self.live => (
                egui::Color32::from_rgb(0xF4, 0xF0, 0xEA),
                theme::SOFT_INK,
                "Last reading".to_string(),
                "Pluck again to check.".to_string(),
            ),
            Some(c) if c.abs() <= 5.0 => (
                theme::GREEN_SOFT,
                green,
                "In tune!".to_string(),
                "Looks good. Keep it up!".to_string(),
            ),
            Some(c) if c < 0.0 => (
                theme::YELLOW_SOFT,
                egui::Color32::from_rgb(0xB0, 0x6A, 0x00),
                format!("{:.0} cents low", -c),
                "Tighten the string a little.".to_string(),
            ),
            Some(c) => (
                theme::ACCENT_SOFT,
                theme::ACCENT_DARK,
                format!("{c:.0} cents high"),
                "Loosen the string a little.".to_string(),
            ),
            None => (
                egui::Color32::from_rgb(0xF4, 0xF0, 0xEA),
                theme::SOFT_INK,
                "Waiting for a note".to_string(),
                "Pluck one string and let it ring.".to_string(),
            ),
        };
        let boxr = egui::Rect::from_center_size(
            Pos2::new(cx, center.y + w * 0.16),
            Vec2::new(w * 0.62, 74.0),
        );
        p.rect_filled(boxr, egui::CornerRadius::same(16), fill);
        let title_pos = Pos2::new(cx + 16.0, boxr.top() + 25.0);
        p.text(
            title_pos,
            Align2::CENTER_CENTER,
            &title,
            theme::heading(21.0),
            theme::INK,
        );
        let tw = p
            .layout_no_wrap(title.clone(), theme::heading(21.0), theme::INK)
            .size()
            .x;
        let badge = Pos2::new(title_pos.x - tw / 2.0 - 20.0, title_pos.y);
        p.circle_filled(badge, 13.0, ink);
        if self.cents.is_some_and(|c| c.abs() <= 5.0) && self.live {
            super::icons::paint(
                p,
                egui::Rect::from_center_size(badge, Vec2::splat(14.0)),
                super::icons::Icon::Check,
                egui::Color32::WHITE,
            );
        }
        p.text(
            Pos2::new(cx, boxr.top() + 54.0),
            Align2::CENTER_CENTER,
            &line,
            FontId::proportional(16.0),
            theme::SOFT_INK,
        );
        response
    }
}
