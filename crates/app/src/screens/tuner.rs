//! The tuner (lesson L0.4): pick a microphone, pluck a string, follow the needle.

use crate::app::App;
use crate::theme;
use crate::widgets::icons::{self, Icon};
use crate::widgets::kit::{self, card};
use crate::widgets::needle::TunerNeedle;
use egui::{Color32, CornerRadius, RichText, Ui, Vec2};
use theory::{NoteName, Spelling};

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| body(app, ui));
}

fn body(app: &mut App, ui: &mut Ui) {
    kit::page_header(
        ui,
        "Tuner",
        "Tune your guitar",
        "Pluck one string at a time and let it ring. Turn the tuning peg slowly until the needle is in the green middle.",
    );
    mic_controls(app, ui);
    if app.tuner.is_none() && app.mic.is_on() {
        app.tuner = Some(audio::Tuner::new(app.mic.sample_rate()));
    }
    if !app.mic.is_on() {
        app.tuner = None;
    }
    ui.add_space(14.0);
    let names = app.tuning.open_names();
    let reading = app.tuner.as_ref().and_then(|t| t.reading());
    let full = ui.available_width();
    let gap = ui.spacing().item_spacing.x;
    let left_w = (full * 0.52).floor();
    let right_w = full - left_w - gap - 1.0;
    ui.horizontal_top(|ui| {
        ui.allocate_ui(Vec2::new(left_w, 0.0), |ui| {
            card(ui, |ui| {
                ui.set_width(left_w - 42.0);
                let (label, sub, cents) = match reading {
                    Some(r) => match r.string {
                        Some((s, c)) if app.tuner_target.is_some() || c.abs() < 100.0 => (
                            names[s].to_string(),
                            format!(
                                "{:.1} Hz · string {}",
                                r.frequency,
                                theory::string_number(s)
                            ),
                            Some(c),
                        ),
                        _ => (
                            theme::glyphs(
                                &NoteName::from_pc(r.note.pc(), Spelling::Sharps).pretty(),
                            ),
                            format!("{:.1} Hz · not close to a string yet", r.frequency),
                            Some(r.cents_from_note),
                        ),
                    },
                    None => (
                        "—".into(),
                        if app.mic.is_on() {
                            "Pluck a string…".into()
                        } else {
                            "Turn on the microphone to start".into()
                        },
                        None,
                    ),
                };
                ui.vertical_centered(|ui| {
                    TunerNeedle {
                        cents,
                        label,
                        sublabel: sub,
                        size: (left_w - 60.0).min(460.0),
                        live: app.tuner.as_ref().is_some_and(|t| t.is_live()),
                    }
                    .show(ui);
                });
            });
        });
        ui.allocate_ui(Vec2::new(right_w, 0.0), |ui| {
            card(ui, |ui| {
                ui.set_width(right_w - 42.0);
                ui.label(RichText::new("Which string?").font(theme::heading(24.0)));
                kit::subtitle(
                    ui,
                    "Auto finds the closest string. Pick one to tune it on purpose.",
                );
                ui.add_space(4.0);
                let auto = app.tuner_target.is_none();
                let kind = if auto {
                    kit::Kind::Soft
                } else {
                    kit::Kind::Secondary
                };
                if kit::button(ui, kind, None, "Auto").clicked() {
                    app.tuner_target = None;
                }
                ui.add_space(6.0);
                // Thinnest string on top, like the neck map.
                for s in (0..6).rev() {
                    let selected = app.tuner_target == Some(s);
                    let in_tune = reading
                        .and_then(|r| r.string)
                        .is_some_and(|(rs, c)| rs == s && audio::TunerReading::in_tune(c));
                    string_row(
                        app,
                        ui,
                        s,
                        &theme::glyphs(&names[s].pretty()),
                        selected,
                        in_tune,
                    );
                }
                ui.add_space(8.0);
                kit::tip(
                    ui,
                    "Tip: tune UP to the note. If you go too high, go below it and come back up.",
                );
            });
        });
    });
}

/// One row of the string list: name, note, frequency, hear it, and pick-to-tune.
fn string_row(app: &mut App, ui: &mut Ui, s: usize, name: &str, selected: bool, in_tune: bool) {
    let freq = app.tuning.open[s].frequency();
    let fill = if selected {
        theme::ACCENT_SOFT
    } else {
        Color32::TRANSPARENT
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let text = RichText::new(format!("String {}", theory::string_number(s))).size(17.0);
                let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                ui.add_space(12.0);
                ui.label(RichText::new(name).size(17.0));
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("{freq:.1} Hz"))
                        .size(16.0)
                        .color(theme::SOFT_INK),
                );
                if in_tune {
                    kit::status_dot(ui, true, theme::GREEN, 20.0);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pick = kit::icon_button(
                        ui,
                        if selected {
                            Icon::Check
                        } else {
                            Icon::ChevronRight
                        },
                        &format!("Tune string {} on purpose", theory::string_number(s)),
                    );
                    if pick.clicked() || resp.clicked() {
                        app.tuner_target = if selected { None } else { Some(s) };
                    }
                    if kit::secondary(ui, Some(Icon::Speaker), "Hear it").clicked() {
                        app.sound.note(&app.tuning, s, 0);
                    }
                });
            });
        });
}

/// Microphone picker, on/off, level meter, and any error message, in a card.
pub fn mic_controls(app: &mut App, ui: &mut Ui) {
    card(ui, |ui| mic_row(app, ui));
}

/// The microphone controls on their own (for placing inside another card).
pub fn mic_row(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        icons::show(ui, Icon::Mic, 24.0, theme::INK);
        ui.label(RichText::new("Microphone").size(17.0));
        let current = app
            .mic
            .selected
            .as_ref()
            .and_then(|id| app.mic.devices.iter().find(|d| &d.id == id))
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "System default".into());
        let mut changed = false;
        egui::ComboBox::from_id_salt("mic_device")
            .selected_text(RichText::new(current).size(16.0))
            .width(240.0)
            .height(300.0)
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut app.mic.selected, None, "System default")
                    .changed();
                for d in app.mic.devices.clone() {
                    changed |= ui
                        .selectable_value(&mut app.mic.selected, Some(d.id.clone()), d.name.clone())
                        .changed();
                }
            });
        if kit::icon_button(ui, Icon::Refresh, "Look for microphones again").clicked() {
            app.mic.refresh_devices();
        }
        if changed && app.mic.is_on() {
            app.mic.stop();
            app.tuner = None;
            app.mic.start(&app.sound);
        }
        if app.mic.is_on() {
            if kit::secondary(ui, Some(Icon::Stop), "Stop listening").clicked() {
                app.mic.stop();
            }
        } else if kit::primary(ui, Some(Icon::Mic), "Start listening").clicked() {
            app.mic.start(&app.sound);
        }
        level_meter(ui, app.mic.level_db, app.mic.is_on());
    });
    if let Some(err) = &app.mic.error {
        ui.colored_label(theme::VERMILLION, err);
    }
}

fn level_meter(ui: &mut Ui, db: f32, on: bool) {
    let hint = if !on {
        "Off"
    } else if db < -45.0 {
        "Quiet: pluck a string"
    } else if db > -6.0 {
        "Very loud"
    } else {
        "Good level"
    };
    // Bar and its label share one allocation so the label never wraps to a new line.
    let bar_w = 200.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_w + 150.0, 32.0), egui::Sense::hover());
    let bar = egui::Rect::from_min_size(
        rect.left_center() - Vec2::new(0.0, 7.0),
        Vec2::new(bar_w, 14.0),
    );
    let p = ui.painter();
    p.rect_filled(
        bar,
        CornerRadius::same(7),
        Color32::from_rgb(0xEF, 0xEA, 0xE2),
    );
    if on {
        let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
        let mut fill = bar;
        fill.set_width(bar.width() * frac);
        let color = if db > -6.0 {
            theme::VERMILLION
        } else if db > -45.0 {
            theme::GREEN
        } else {
            Color32::from_gray(170)
        };
        p.rect_filled(fill, CornerRadius::same(7), color);
    }
    p.text(
        bar.right_center() + Vec2::new(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        hint,
        egui::FontId::proportional(14.5),
        theme::SOFT_INK,
    );
}
