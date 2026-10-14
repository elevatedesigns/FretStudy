//! Choose or create a profile. Profiles are local: no accounts, no passwords, optional PIN.

use super::card;
use crate::app::App;
use crate::theme;
use crate::widgets::kit;
use egui::{RichText, Ui};
use store::AgeBand;

/// Only emoji that the built-in emoji font can draw.
pub const AVATARS: [&str; 16] = [
    "🐱", "🐶", "🐻", "🐼", "🐸", "🐙", "🐢", "🐝", "🐧", "🐳", "🐰", "🐯", "🎸", "🎵", "⭐", "🚀",
];

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    List,
    New,
    Pin(i64),
}

#[derive(Default)]
pub struct ProfilesUi {
    mode: Mode,
    name: String,
    avatar: usize,
    age: Option<AgeBand>,
    use_pin: bool,
    pin: String,
    pin_entry: String,
    error: Option<String>,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("FretStudy")
                .font(theme::logo(54.0))
                .color(theme::ACCENT),
        );
        ui.label(
            RichText::new("Learn guitar, one building block at a time.")
                .size(20.0)
                .color(theme::SOFT_INK),
        );
    });
    ui.add_space(20.0);
    let profiles = app
        .store
        .as_ref()
        .and_then(|s| s.profiles().ok())
        .unwrap_or_default();
    if profiles.is_empty() && app.profiles_ui.mode == Mode::List {
        app.profiles_ui.mode = Mode::New;
    }
    match app.profiles_ui.mode {
        Mode::List => {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Who's playing?").font(theme::heading(28.0)))
            });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for p in &profiles {
                    let resp = card(ui, |ui| {
                        ui.set_width(170.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(&p.avatar).size(56.0));
                            ui.label(RichText::new(&p.name).font(theme::heading(22.0)));
                            if p.has_pin {
                                ui.label(RichText::new("🔒 PIN").small().color(theme::SOFT_INK));
                            }
                            kit::secondary(ui, None, "Play ▶").clicked()
                        })
                        .inner
                    });
                    if resp {
                        if p.has_pin {
                            app.profiles_ui.mode = Mode::Pin(p.id);
                            app.profiles_ui.pin_entry.clear();
                            app.profiles_ui.error = None;
                        } else {
                            app.load_profile(p.id);
                        }
                    }
                }
                let new = card(ui, |ui| {
                    ui.set_width(170.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("➕").size(56.0));
                        ui.label(RichText::new("New player").font(theme::heading(22.0)));
                        kit::secondary(ui, None, "Create").clicked()
                    })
                    .inner
                });
                if new {
                    app.profiles_ui = ProfilesUi {
                        mode: Mode::New,
                        ..Default::default()
                    };
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .button("📥 Import a backup…")
                    .on_hover_text(
                        "Bring in a player from a .fretstudy file (made with Back up in Settings)",
                    )
                    .clicked()
                {
                    app.profiles_ui.error = import_backup(app);
                }
                if let Some(e) = &app.profiles_ui.error {
                    ui.colored_label(theme::VERMILLION, e);
                }
            });
        }
        Mode::Pin(id) => {
            let name = profiles
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            card(ui, |ui| {
                ui.set_max_width(420.0);
                ui.label(RichText::new(format!("Enter {name}'s PIN")).font(theme::heading(24.0)));
                let r = ui.add(
                    egui::TextEdit::singleline(&mut app.profiles_ui.pin_entry)
                        .password(true)
                        .hint_text("4 digits")
                        .desired_width(160.0)
                        .font(theme::heading(24.0)),
                );
                r.request_focus();
                let submit = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() || submit {
                        let ok = app.store.as_ref().is_some_and(|s| {
                            s.check_pin(id, &app.profiles_ui.pin_entry).unwrap_or(false)
                        });
                        if ok {
                            app.load_profile(id);
                        } else {
                            app.profiles_ui.error = Some("That PIN doesn't match.".into());
                        }
                    }
                    if ui.button("Back").clicked() {
                        app.profiles_ui.mode = Mode::List;
                    }
                });
                if let Some(e) = &app.profiles_ui.error {
                    ui.colored_label(theme::VERMILLION, e);
                }
            });
        }
        Mode::New => new_profile(app, ui, !profiles.is_empty()),
    }
}

fn new_profile(app: &mut App, ui: &mut Ui, can_go_back: bool) {
    card(ui, |ui| {
        ui.set_max_width(640.0);
        ui.label(RichText::new("Make your player profile").font(theme::heading(26.0)));
        ui.label(RichText::new("Everything stays on this computer. Each person in your family can have their own profile.").color(theme::SOFT_INK));
        ui.add_space(8.0);
        let u = &mut app.profiles_ui;
        ui.label(RichText::new("Your name").strong());
        ui.add(
            egui::TextEdit::singleline(&mut u.name)
                .hint_text("First name or nickname")
                .desired_width(280.0)
                .font(egui::FontId::proportional(20.0)),
        );
        ui.add_space(6.0);
        ui.label(RichText::new("Pick a picture").strong());
        ui.horizontal_wrapped(|ui| {
            for (i, a) in AVATARS.iter().enumerate() {
                if ui
                    .selectable_label(u.avatar == i, RichText::new(*a).size(30.0))
                    .clicked()
                {
                    u.avatar = i;
                }
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Who's learning?").strong());
        ui.horizontal(|ui| {
            for (band, label) in [
                (AgeBand::Kid, "A kid"),
                (AgeBand::Teen, "A teen"),
                (AgeBand::Adult, "An adult"),
            ] {
                if ui
                    .selectable_label(u.age == Some(band), RichText::new(label).size(17.0))
                    .clicked()
                {
                    u.age = Some(band);
                }
            }
        });
        ui.label(RichText::new("This sets a playful or calmer look. Lessons are the same for everyone, and you can change it later.").small().color(theme::SOFT_INK));
        ui.add_space(6.0);
        ui.checkbox(
            &mut u.use_pin,
            "Add a 4-digit PIN (keeps siblings out of each other's progress)",
        );
        if u.use_pin {
            ui.add(
                egui::TextEdit::singleline(&mut u.pin)
                    .password(true)
                    .hint_text("4 digits")
                    .desired_width(120.0),
            );
        }
        if let Some(e) = &u.error {
            ui.colored_label(theme::VERMILLION, e);
        }
        ui.add_space(8.0);
        let mut create = false;
        ui.horizontal(|ui| {
            create = kit::primary(ui, None, "Let's go! ▶").clicked();
            if can_go_back && ui.button("Back").clicked() {
                u.mode = Mode::List;
            }
        });
        if create {
            let name = u.name.trim().to_string();
            let pin_ok =
                !u.use_pin || (u.pin.len() == 4 && u.pin.chars().all(|c| c.is_ascii_digit()));
            if name.is_empty() {
                u.error = Some("Please type a name.".into());
            } else if u.age.is_none() {
                u.error = Some("Please choose who's learning.".into());
            } else if !pin_ok {
                u.error = Some("A PIN needs exactly 4 digits.".into());
            } else {
                let pin = u.use_pin.then(|| u.pin.clone());
                let result = app.store.as_ref().map(|s| {
                    s.create_profile(&name, AVATARS[u.avatar], u.age.unwrap(), pin.as_deref())
                });
                match result {
                    Some(Ok(id)) => app.load_profile(id),
                    Some(Err(e)) => {
                        app.profiles_ui.error = Some(format!("Couldn't save the profile: {e:#}"))
                    }
                    None => app.profiles_ui.error = Some("Saving isn't available.".into()),
                }
            }
        }
    });
}

/// Asks for a `.fretstudy` file and adds it as a player. Returns an error message, if any.
fn import_backup(app: &mut App) -> Option<String> {
    let path = rfd::FileDialog::new()
        .add_filter("FretStudy backup", &["fretstudy"])
        .pick_file()?;
    let store = app.store.as_ref()?;
    match store.import_profile(&path) {
        Ok(_) => None,
        Err(e) => Some(format!("Couldn't import that file: {e:#}")),
    }
}
