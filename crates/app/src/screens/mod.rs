pub mod chords;
pub mod neck;
pub mod path;
pub mod placement;
pub mod practice;
pub mod profiles;
pub mod settings;
pub mod songs;
pub mod sound_check;
pub mod tuner;
pub mod welcome;

use crate::theme;
use crate::widgets::{ColorMode, DotShape};
use egui::{CornerRadius, Rect, RichText, Ui, Vec2};
use theory::Role;

/// A small color key so learners always know what the colors mean.
pub fn legend(ui: &mut Ui, mode: ColorMode) {
    let items: Vec<(egui::Color32, &str, DotShape)> = match mode {
        ColorMode::Finger => vec![
            (theme::finger_color(1), "1 index", DotShape::Circle),
            (theme::finger_color(2), "2 middle", DotShape::Circle),
            (theme::finger_color(3), "3 ring", DotShape::Circle),
            (theme::finger_color(4), "4 pinky", DotShape::Circle),
        ],
        ColorMode::Role => vec![
            (
                theme::role_color(Role::Root),
                "Root (home)",
                DotShape::Square,
            ),
            (theme::role_color(Role::Third), "3rd", DotShape::Circle),
            (theme::role_color(Role::Fifth), "5th", DotShape::Circle),
            (theme::role_color(Role::Seventh), "7th", DotShape::Circle),
            (theme::role_color(Role::Other), "other", DotShape::Circle),
            (
                theme::role_color(Role::BlueNote),
                "blue note",
                DotShape::Diamond,
            ),
        ],
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(if mode == ColorMode::Finger {
                "Finger colors:"
            } else {
                "Note jobs:"
            })
            .color(theme::SOFT_INK)
            .small(),
        );
        for (color, text, shape) in items {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
            let c = rect.center();
            match shape {
                DotShape::Square => {
                    ui.painter().rect_filled(
                        Rect::from_center_size(c, Vec2::splat(13.0)),
                        CornerRadius::same(3),
                        color,
                    );
                }
                DotShape::Diamond => {
                    let d = 7.5;
                    let pts = vec![
                        c + Vec2::new(0.0, -d),
                        c + Vec2::new(d, 0.0),
                        c + Vec2::new(0.0, d),
                        c + Vec2::new(-d, 0.0),
                    ];
                    ui.painter()
                        .add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
                }
                DotShape::Circle => {
                    ui.painter().circle_filled(c, 7.0, color);
                }
            }
            ui.label(RichText::new(text).small());
            ui.add_space(6.0);
        }
    });
}

/// A rounded card section (the shared card style).
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    crate::widgets::kit::card(ui, add)
}
