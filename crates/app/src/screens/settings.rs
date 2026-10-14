//! Profile and app settings.

use super::tuner::mic_row;
use crate::app::{App, Screen};
use crate::theme;
use crate::widgets::icons::{self, Icon};
use crate::widgets::kit::{self, Kind, card};
use egui::{Color32, RichText, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(profile) = app.profile.clone() else {
        return;
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        kit::page_header(
            ui,
            "Settings",
            &format!("{}'s settings", profile.name),
            "Customize your learning experience, audio setup, and preferences to make FretStudy work best for you.",
        );
        card(ui, |ui| {
            ui.label(RichText::new("Look and feel").font(theme::heading(24.0)));
            kit::subtitle(ui, "Choose a style and how FretStudy looks.");
            ui.add_space(6.0);
            kit::inset(ui, Color32::from_rgb(0xFB, 0xF8, 0xF3), |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(p) = app.profile.as_mut() {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Vibe").strong());
                            kit::segmented(
                                ui,
                                &mut p.settings.look,
                                &[("playful".to_string(), "🎉 Playful"), ("clean".to_string(), "Clean and calm")],
                            );
                        });
                        ui.add_space(30.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Text size").strong());
                            ui.add(egui::Slider::new(&mut p.settings.text_scale, 0.8..=1.5).step_by(0.05).fixed_decimals(2));
                        });
                    }
                });
                ui.separator();
                ui.label(RichText::new("Diagrams").strong());
                ui.label(RichText::new("How chord boxes, necks, and tab look throughout the app.").size(14.5).color(theme::SOFT_INK));
                app.diagram_controls(ui, true);
            });
        });
        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(RichText::new("Microphone").font(theme::heading(24.0)));
            kit::subtitle(ui, "Pick the microphone FretStudy listens with. Run Sound Check so rhythm lessons can judge your timing fairly.");
            ui.add_space(6.0);
            mic_row(app, ui);
            match app.latency_ms {
                Some(ms) => ui.label(RichText::new(format!("Measured audio delay: {ms:.0} ms (from Sound Check).")).color(theme::SOFT_INK)),
                None => ui.label(RichText::new("Audio delay not measured yet. Run Sound Check once.").color(theme::SOFT_INK)),
            };
        });
        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(RichText::new("Your path").font(theme::heading(24.0)));
            kit::subtitle(ui, "Retake the placement quiz to move your starting point. Lessons you've completed stay completed.");
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(0xFF, 0xF6, 0xEF))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(52.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 26.0, theme::ACCENT_SOFT);
                        icons::paint(ui.painter(), r.shrink(13.0), Icon::Target, theme::ACCENT);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Retake the placement quiz").font(theme::heading(18.0)));
                            ui.label(RichText::new("Sets your starting point from your current skill level.").size(15.0).color(theme::SOFT_INK));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if kit::primary(ui, None, "Retake the placement quiz").clicked() {
                                app.placement = Some(super::placement::PlacementUi::new());
                                app.screen = Screen::Placement;
                            }
                        });
                    });
                });
        });
        ui.add_space(14.0);
        let full = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        let left_w = (full * 0.55).floor();
        let right_w = full - left_w - gap - 1.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(left_w, 0.0), |ui| profiles_card(app, ui, &profile));
            ui.allocate_ui(Vec2::new(right_w, 0.0), |ui| {
                card(ui, |ui| {
                    ui.label(RichText::new("About").font(theme::heading(24.0)));
                    ui.label(RichText::new("FretStudy is a free guitar course. It keeps everything on this computer. Nothing is sent anywhere.").size(15.0).color(theme::SOFT_INK));
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14.0).color(theme::SOFT_INK));
                });
            });
        });
    });
}

fn profiles_card(app: &mut App, ui: &mut Ui, profile: &store::Profile) {
    card(ui, |ui| {
        ui.label(RichText::new("Profiles").font(theme::heading(24.0)));
        kit::subtitle(
            ui,
            "Manage the players on this computer, and back up this one.",
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if kit::secondary(ui, Some(Icon::Users), "Switch player").clicked() {
                app.sign_out();
            }
            if kit::secondary(ui, Some(Icon::Download), "Back up this profile…")
                .on_hover_text("Save progress, songs, and recordings to one file you can move to another computer")
                .clicked()
            {
                let msg = backup(app, profile);
                ui.data_mut(|d| d.insert_temp(egui::Id::new("backup_msg"), msg));
            }
        });
        if app.profile.is_none() {
            return;
        }
        if let Some(msg) = ui.data(|d| d.get_temp::<String>(egui::Id::new("backup_msg")))
            && !msg.is_empty()
        {
            ui.label(RichText::new(msg).color(theme::GREEN));
        }
        ui.add_space(6.0);
        let confirm_id = egui::Id::new("confirm_delete");
        let confirming = ui.data(|d| d.get_temp::<bool>(confirm_id)).unwrap_or(false);
        if !confirming {
            if kit::button(ui, Kind::Danger, Some(Icon::Trash), "Delete this profile…").clicked()
            {
                ui.data_mut(|d| d.insert_temp(confirm_id, true));
            }
        } else {
            ui.label(
                RichText::new(format!(
                    "Delete {} and all of their progress? This can't be undone.",
                    profile.name
                ))
                .color(theme::VERMILLION),
            );
            ui.horizontal(|ui| {
                if kit::button(ui, Kind::Danger, Some(Icon::Trash), "Yes, delete").clicked() {
                    if let Some(store) = &app.store {
                        let _ = store.delete_profile(profile.id);
                    }
                    ui.data_mut(|d| d.insert_temp(confirm_id, false));
                    app.sign_out();
                }
                if kit::secondary(ui, None, "Cancel").clicked() {
                    ui.data_mut(|d| d.insert_temp(confirm_id, false));
                }
            });
        }
    });
}

/// Saves the profile to a `.fretstudy` file the learner picks.
fn backup(app: &App, profile: &store::Profile) -> String {
    let Some(store) = &app.store else {
        return String::new();
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{}.fretstudy", profile.name))
        .add_filter("FretStudy backup", &["fretstudy"])
        .save_file()
    else {
        return String::new();
    };
    match store.export_profile(profile.id, &path) {
        Ok(()) => format!(
            "Saved to {}. On the other computer, choose Import a backup on the player screen.",
            path.display()
        ),
        Err(e) => format!("Couldn't save the backup: {e:#}"),
    }
}
