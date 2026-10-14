//! First-time welcome: start at the beginning, or take the quick placement quiz.

use super::card;
use crate::app::{App, Screen};
use crate::theme;
use crate::widgets::kit;
use egui::{RichText, Ui};

pub fn show(app: &mut App, ui: &mut Ui) {
    let name = app
        .profile
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    ui.add_space(16.0);
    ui.label(RichText::new(format!("Hi {name}! 👋")).font(theme::heading(40.0)));
    ui.label(RichText::new("Let's find the best place for you to start.").size(22.0));
    ui.add_space(16.0);
    ui.horizontal_top(|ui| {
        card(ui, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("🌱 I'm brand new").font(theme::heading(24.0)));
            ui.label(RichText::new("Start at the very beginning: meet your guitar, tune it, and play your first chord.").size(17.0));
            ui.add_space(8.0);
            if kit::primary(ui, None, "Start at the beginning ▶").clicked() {
                app.complete_placement(&[]);
                app.screen = Screen::Path;
            }
        });
        card(ui, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("🎸 I've played before").font(theme::heading(24.0)));
            ui.label(RichText::new("Take a quick quiz (about 3 minutes, 12 questions or fewer). We'll skip what you already know — you can always go back.").size(17.0));
            ui.add_space(8.0);
            if kit::primary(ui, None, "Take the quiz ▶").clicked() {
                app.placement = Some(super::placement::PlacementUi::new());
                app.screen = Screen::Placement;
            }
        });
    });
    ui.add_space(12.0);
    ui.label(
        RichText::new("Tip: visit 🔊 Sound Check first so FretStudy can hear your guitar.")
            .color(theme::SOFT_INK),
    );
}
