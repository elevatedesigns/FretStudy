//! Record yourself: a take from the microphone is saved as a WAV file next to the
//! progress database, listed with its lesson, and played back through the speakers.

use crate::app::App;
use crate::screens::tuner::mic_controls;
use crate::theme;
use egui::{RichText, Ui};
use std::sync::Arc;

/// Longest take kept (keeps memory use sensible).
const MAX_SECONDS: f32 = 300.0;

#[derive(Default)]
pub struct RecorderUi {
    pub error: Option<String>,
    /// Recordings shown in the list (refreshed after each change).
    takes: Option<Vec<store::Recording>>,
}

impl RecorderUi {
    /// Starts a take (the mic must be on).
    pub fn start(app: &mut App) {
        app.mic.recording = Some(Vec::new());
    }

    /// Ends the take and returns its samples.
    pub fn stop(app: &mut App) -> Option<Vec<f32>> {
        app.mic.recording.take()
    }

    /// Saves samples as a WAV and remembers them for this profile.
    pub fn save(&mut self, app: &App, lesson: Option<&str>, samples: &[f32], rate: u32) {
        let (Some(store), Some(p)) = (&app.store, &app.profile) else {
            return;
        };
        let result = store.new_recording_path(p.id).and_then(|path| {
            audio::wav::write(&path, samples, rate)?;
            store.add_recording(p.id, lesson, &path, samples.len() as f32 / rate as f32)
        });
        if let Err(e) = result {
            self.error = Some(format!("Couldn't save the recording: {e:#}"));
        }
        self.takes = None;
    }

    /// The record button, timer, and (when `list`) this lesson's saved takes.
    /// `save` writes each finished take to disk.
    pub fn panel(
        &mut self,
        app: &mut App,
        ui: &mut Ui,
        lesson: Option<&str>,
        save: bool,
        list: bool,
    ) {
        if !app.mic.is_on() {
            ui.label(RichText::new("Turn on the microphone to record.").color(theme::SOFT_INK));
            mic_controls(app, ui);
        } else {
            let rate = app.mic.sample_rate();
            let recorded = app
                .mic
                .recording
                .as_ref()
                .map(|r| r.len() as f32 / rate as f32);
            ui.horizontal(|ui| match recorded {
                None => {
                    if ui
                        .button(
                            RichText::new("⏺ Record")
                                .size(18.0)
                                .color(theme::VERMILLION),
                        )
                        .clicked()
                    {
                        Self::start(app);
                    }
                }
                Some(secs) => {
                    let stop = ui.button(RichText::new("■ Stop").size(18.0)).clicked()
                        || secs >= MAX_SECONDS;
                    ui.label(
                        RichText::new(format!(
                            "⏺ Recording… {}:{:02}",
                            secs as u32 / 60,
                            secs as u32 % 60
                        ))
                        .color(theme::VERMILLION)
                        .size(18.0),
                    );
                    ui.ctx().request_repaint();
                    if stop
                        && let Some(samples) = Self::stop(app)
                        && save
                        && !samples.is_empty()
                    {
                        self.save(app, lesson, &samples, rate);
                    }
                }
            });
        }
        if let Some(e) = &self.error {
            ui.colored_label(theme::VERMILLION, e);
        }
        if list {
            self.list(app, ui, lesson);
        }
    }

    /// Saved takes with play, show-file, and delete buttons.
    pub fn list(&mut self, app: &mut App, ui: &mut Ui, lesson: Option<&str>) {
        let (Some(store), Some(p)) = (&app.store, &app.profile) else {
            return;
        };
        if self.takes.is_none() {
            self.takes = store.recordings(p.id, lesson).ok();
        }
        let takes = self.takes.clone().unwrap_or_default();
        if takes.is_empty() {
            return;
        }
        ui.label(RichText::new("Your takes").font(theme::heading(18.0)));
        let mut changed = false;
        for (i, t) in takes.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Take {} · {}:{:02}",
                    takes.len() - i,
                    t.seconds as u32 / 60,
                    t.seconds as u32 % 60
                ));
                if ui.button("▶ Play").clicked() {
                    match audio::wav::read(&t.path) {
                        Ok((samples, rate)) => {
                            app.sound.cancel_scheduled();
                            app.sound.play_recording(Arc::from(samples), rate);
                        }
                        Err(e) => self.error = Some(format!("{e:#}")),
                    }
                }
                if ui.button("■").on_hover_text("Stop playback").clicked() {
                    app.sound.cancel_scheduled();
                }
                if ui.button("📂 Show file").clicked() {
                    reveal(&t.path);
                }
                if ui.button("🗑").on_hover_text("Delete this take").clicked()
                    && let Some(store) = &app.store
                {
                    let _ = store.delete_recording(t.id);
                    changed = true;
                }
            });
        }
        if changed {
            self.takes = None;
        }
    }
}

/// Shows a file in Finder / Explorer.
pub fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
}
