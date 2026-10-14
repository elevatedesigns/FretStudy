//! Sound Check (first-time setup): confirms sound plays, the mic works, and measures
//! the delay between what the app plays and what the mic hears (for rhythm checks).

use super::tuner::mic_row;
use crate::app::App;
use crate::sound::Calibration;
use crate::theme;
use crate::widgets::icons::{self, Icon};
use crate::widgets::kit::{self, card};
use egui::{Color32, RichText, Ui};
use theory::library;

/// A numbered step: the big circle on the left, the content on the right.
fn step(ui: &mut Ui, n: usize, title: &str, add: impl FnOnce(&mut Ui)) {
    card(ui, |ui| {
        ui.horizontal_top(|ui| {
            kit::step_badge(ui, n);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).font(theme::heading(26.0)));
                add(ui);
            });
        });
    });
    ui.add_space(14.0);
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        kit::page_header(
            ui,
            "Tools",
            "Sound Check",
            "Let's make sure FretStudy can play sounds and hear your guitar. This takes about a minute.",
        );
        step(ui, 1, "Can you hear the app?", |ui| {
            kit::subtitle(ui, "We'll play a G chord so you can confirm FretStudy can play sound.");
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if kit::primary(ui, Some(Icon::Speaker), "Play a G chord").clicked()
                    && let Some(g) = library::find("G")
                {
                    app.sound.strum(&g.shape(), &app.tuning, audio::play::StrumDirection::Down);
                }
                ui.add_space(16.0);
                match app.sound.engine() {
                    Some(engine) => {
                        kit::inset(ui, Color32::from_rgb(0xF8, 0xF4, 0xEE), |ui| {
                            ui.horizontal(|ui| {
                                icons::show(ui, Icon::Guitar, 26.0, theme::INK);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Playing through").size(14.0).color(theme::SOFT_INK));
                                    ui.label(
                                        RichText::new(format!(
                                            "{} ({} Hz, about {:.0} ms output delay)",
                                            engine.device_name(),
                                            engine.sample_rate(),
                                            engine.clock().output_latency_ms()
                                        ))
                                        .strong(),
                                    );
                                });
                            });
                        });
                    }
                    None => {
                        ui.colored_label(theme::VERMILLION, app.sound.error.clone().unwrap_or_default());
                    }
                }
            });
            ui.label(RichText::new("If you don't hear the chord, check your volume and output device, then try again.").size(14.5).color(theme::SOFT_INK));
        });

        step(ui, 2, "Turn on your microphone", |ui| {
            kit::subtitle(ui, "We'll listen for your guitar so FretStudy can give you feedback.");
            ui.add_space(6.0);
            mic_row(app, ui);
            ui.label(RichText::new("Allow microphone access if your computer asks.").size(14.5).color(theme::SOFT_INK));
        });

        step(ui, 3, "Timing check", |ui| {
            kit::subtitle(ui, "FretStudy will play 8 clicks and listen for them. This measures the tiny delay in your computer's audio so rhythm lessons can judge your timing fairly.");
            ui.add_space(6.0);
            kit::inset(ui, theme::BLUE_SOFT, |ui| {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::Headphones, 28.0, theme::INK);
                    ui.vertical(|ui| {
                        kit::toggle(
                            ui,
                            &mut app.calibration.strum_along,
                            "I'm wearing headphones (the mic can't hear the clicks), so I'll strum along instead",
                        );
                        let hint = if app.calibration.strum_along {
                            "Mute the strings with your fretting hand and strum a short \"chk\" exactly on each click."
                        } else {
                            "Use your speakers, turn the volume up, and keep the room quiet."
                        };
                        ui.label(RichText::new(hint).size(14.5).color(theme::SOFT_INK));
                    });
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let can_run = app.mic.is_on() && app.sound.engine().is_some() && !app.calibration.running;
                let resp = ui.add_enabled_ui(can_run, |ui| kit::primary(ui, Some(Icon::Metronome), "Start timing check")).inner;
                if resp.clicked() {
                    app.calibration.start(&mut app.sound, &app.mic);
                }
                if !app.mic.is_on() {
                    ui.label(RichText::new("Turn on the microphone first.").color(theme::SOFT_INK));
                }
            });
            if app.calibration.running {
                let p = app.calibration.progress(app.sound.now_ns());
                kit::progress_bar(ui, p, theme::ACCENT, 360.0);
                ui.label(format!(
                    "Listening… heard {} of {}",
                    app.calibration.onsets_ns.len().min(Calibration::CLICKS),
                    Calibration::CLICKS
                ));
            }
            match &app.calibration.result {
                Some(Ok(ms)) => {
                    ui.label(RichText::new(format!("✔ Done! Your audio delay is about {ms:.0} ms. FretStudy will correct for it.")).color(theme::GREEN).size(17.0));
                }
                Some(Err(e)) => {
                    ui.colored_label(theme::VERMILLION, e);
                }
                None => {}
            }
            if let Some(ms) = app.latency_ms {
                ui.label(RichText::new(format!("Saved delay: {ms:.0} ms")).small().color(theme::SOFT_INK));
            }
        });
    });
}
