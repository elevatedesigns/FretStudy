//! The application shell: profiles, navigation, saved settings, and per-frame audio polling.

use crate::lesson::LessonRunner;
use crate::screens;
use crate::sound::{Calibration, Mic, Sound};
use crate::theme;
use crate::widgets::{ColorMode, DiagramStyle, LabelMode};
use content::{Curriculum, Done};
use egui::{RichText, Ui};
use std::collections::HashMap;
use std::sync::Arc;
use store::{Settings, Store};
use theory::{ScaleKind, Tuning};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Profiles,
    Welcome,
    Placement,
    Path,
    Lesson,
    Practice,
    Songs,
    Chords,
    Neck,
    Tuner,
    SoundCheck,
    Settings,
}

/// Which notes the Neck Explorer shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeckView {
    WholeNeck,
    /// Pentatonic box 1–5.
    Box(usize),
}

pub struct NeckState {
    pub root: u8,
    pub kind: ScaleKind,
    pub view: NeckView,
    pub last_clicked: Option<(usize, u8)>,
}

pub struct App {
    pub sound: Sound,
    pub mic: Mic,
    pub tuner: Option<audio::Tuner>,
    pub tuner_target: Option<usize>,
    pub calibration: Calibration,
    pub latency_ms: Option<f32>,
    pub tuning: Tuning,
    pub style: DiagramStyle,
    pub screen: Screen,
    pub chord_id: &'static str,
    pub chord_view: screens::chords::ChordView,
    pub neck: NeckState,

    pub store: Option<Store>,
    pub store_error: Option<String>,
    pub curriculum: Arc<Curriculum>,
    pub profile: Option<store::Profile>,
    pub progress: HashMap<String, Done>,
    pub lesson: Option<LessonRunner>,
    /// Open this lesson at the end of the frame (e.g. "Next lesson" button).
    pub pending_open: Option<String>,
    /// Close the current lesson at the end of the frame.
    pub close_lesson: bool,
    pub profiles_ui: screens::profiles::ProfilesUi,
    pub placement: Option<screens::placement::PlacementUi>,
    pub practice: screens::practice::PracticeUi,
    pub songs: screens::songs::SongsUi,
    applied_zoom: f32,
    saved_settings: Option<Settings>,
}

fn style_from(settings: &Settings) -> DiagramStyle {
    DiagramStyle {
        color_mode: if settings.color_mode == "role" {
            ColorMode::Role
        } else {
            ColorMode::Finger
        },
        label_mode: match settings.label_mode.as_str() {
            "note" => LabelMode::Note,
            "interval" => LabelMode::Interval,
            _ => LabelMode::Finger,
        },
        left_handed: settings.left_handed,
        show_staff: settings.show_staff,
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let store = Store::default_path().and_then(|p| Store::open(&p));
        let (store, err) = match store {
            Ok(s) => (Some(s), None),
            // Keep working, but warn that nothing will be saved after the app closes.
            Err(e) => (
                Store::open_in_memory().ok(),
                Some(format!(
                    "Progress can't be saved on this computer, so it will be lost when FretStudy closes ({e:#})."
                )),
            ),
        };
        let mut app = Self::with(cc, Sound::start(), store);
        app.store_error = err;
        app
    }

    pub fn with(cc: &eframe::CreationContext<'_>, sound: Sound, store: Option<Store>) -> App {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply_style(&cc.egui_ctx);
        let mut mic = Mic::new();
        mic.refresh_devices();
        let curriculum =
            Arc::new(Curriculum::load().expect("built-in lessons are valid (checked by tests)"));
        App {
            sound,
            mic,
            tuner: None,
            tuner_target: None,
            calibration: Calibration::new(),
            latency_ms: None,
            tuning: Tuning::standard(),
            style: DiagramStyle::default(),
            screen: Screen::Profiles,
            chord_id: "C",
            chord_view: Default::default(),
            neck: NeckState {
                root: 9,
                kind: ScaleKind::MinorPentatonic,
                view: NeckView::Box(1),
                last_clicked: None,
            },
            store,
            store_error: None,
            curriculum,
            profile: None,
            progress: HashMap::new(),
            lesson: None,
            pending_open: None,
            close_lesson: false,
            profiles_ui: Default::default(),
            placement: None,
            practice: Default::default(),
            songs: Default::default(),
            applied_zoom: 1.0,
            saved_settings: None,
        }
    }

    // ---------------------------------------------------------------- profiles

    pub fn load_profile(&mut self, id: i64) {
        let Some(store) = &self.store else { return };
        let Ok(Some(profile)) = store.profile(id) else {
            return;
        };
        self.style = style_from(&profile.settings);
        self.latency_ms = profile.settings.latency_ms;
        self.mic.selected = profile.settings.input_device.clone();
        self.screen = if profile.placement_done {
            Screen::Path
        } else {
            Screen::Welcome
        };
        self.saved_settings = Some(profile.settings.clone());
        self.profile = Some(profile);
        self.reload_progress();
    }

    pub fn sign_out(&mut self) {
        self.sound.cancel_scheduled();
        self.mic.stop();
        self.tuner = None;
        self.lesson = None;
        self.placement = None;
        self.profile = None;
        self.progress.clear();
        self.profiles_ui = Default::default();
        self.screen = Screen::Profiles;
    }

    pub fn reload_progress(&mut self) {
        self.progress.clear();
        let (Some(store), Some(p)) = (&self.store, &self.profile) else {
            return;
        };
        if let Ok(rows) = store.progress(p.id) {
            for (id, row) in rows {
                if let Some(d) = Done::parse(&row.status) {
                    self.progress.insert(id, d);
                }
            }
        }
    }

    pub fn playful(&self) -> bool {
        self.profile
            .as_ref()
            .is_none_or(|p| p.settings.look == "playful")
    }

    /// Saves settings when anything changed this frame.
    fn sync_settings(&mut self, ctx: &egui::Context) {
        let Some(profile) = self.profile.as_mut() else {
            return;
        };
        let s = &mut profile.settings;
        s.left_handed = self.style.left_handed;
        s.show_staff = self.style.show_staff;
        s.color_mode = if self.style.color_mode == ColorMode::Role {
            "role"
        } else {
            "finger"
        }
        .into();
        s.label_mode = match self.style.label_mode {
            LabelMode::Finger => "finger",
            LabelMode::Note => "note",
            LabelMode::Interval => "interval",
        }
        .into();
        s.latency_ms = self.latency_ms;
        s.input_device = self.mic.selected.clone();
        if self.saved_settings.as_ref() != Some(s) {
            if let Some(store) = &self.store {
                let _ = store.save_settings(profile.id, s);
            }
            self.saved_settings = Some(s.clone());
        }
        let zoom = s.text_scale.clamp(0.8, 1.6);
        if (zoom - self.applied_zoom).abs() > 1e-3 {
            ctx.set_zoom_factor(zoom);
            self.applied_zoom = zoom;
        }
    }

    // ---------------------------------------------------------------- lessons

    pub fn open_lesson(&mut self, id: &str) {
        let Some(lesson) = self.curriculum.lesson(id).cloned() else {
            return;
        };
        if lesson.coming_soon {
            return;
        }
        self.sound.cancel_scheduled();
        if let (Some(store), Some(p)) = (&self.store, &self.profile)
            && !self.progress.get(id).is_some_and(|d| d.satisfies())
        {
            let _ = store.set_status(p.id, id, "in_progress", None);
        }
        self.reload_progress();
        self.lesson = Some(LessonRunner::new(lesson));
        self.screen = Screen::Lesson;
    }

    pub fn finish_lesson(&mut self, id: &str, score: f32, passed: bool, seconds: u32) {
        if let (Some(store), Some(p)) = (&self.store, &self.profile) {
            let _ = store.record_check(p.id, id, score, passed);
            if passed {
                let _ = store.set_status(p.id, id, "completed", Some(score));
                let _ = store.schedule_review(p.id, id, store::day_number());
            }
            let _ = store.log_practice(p.id, &store::today(), seconds, Some(id));
        }
        self.reload_progress();
    }

    pub fn next_lesson_after(&self, _id: &str) -> Option<String> {
        self.curriculum
            .next_lesson(&self.progress)
            .map(str::to_string)
    }

    /// Lessons due for review today: (lesson, stage).
    pub fn due_reviews(&self) -> Vec<(String, u8)> {
        let (Some(store), Some(p)) = (&self.store, &self.profile) else {
            return Vec::new();
        };
        store
            .due_reviews(p.id, store::day_number())
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| self.curriculum.lesson(id).is_some())
            .collect()
    }

    pub fn record_review(&self, lesson: &str, passed: bool) {
        if let (Some(store), Some(p)) = (&self.store, &self.profile) {
            let _ = store.review_result(p.id, lesson, passed, store::day_number());
        }
    }

    /// Practice seconds per day for the last `days` days that had practice.
    pub fn practice_days(&self, days: usize) -> Vec<(String, i64)> {
        let (Some(store), Some(p)) = (&self.store, &self.profile) else {
            return Vec::new();
        };
        store.practice_days(p.id, days).unwrap_or_default()
    }

    pub fn best_changes(&self, a: &str, b: &str) -> Option<u32> {
        let (store, p) = (self.store.as_ref()?, self.profile.as_ref()?);
        store.best_changes(p.id, a, b).ok().flatten()
    }

    pub fn record_changes(&self, a: &str, b: &str, count: u32) {
        if let (Some(store), Some(p)) = (&self.store, &self.profile) {
            let _ = store.record_changes(p.id, a, b, count);
        }
    }

    /// After the placement quiz: mark known lessons and go to the path.
    pub fn complete_placement(&mut self, placed_out: &[String]) {
        if let (Some(store), Some(p)) = (&self.store, &self.profile) {
            let _ = store.clear_placed_out(p.id);
            let _ = store.place_out(p.id, placed_out);
            let _ = store.set_placement_done(p.id, true);
        }
        if let Some(p) = self.profile.as_mut() {
            p.placement_done = true;
        }
        self.reload_progress();
    }

    /// Shared "how should diagrams look" controls.
    pub fn diagram_controls(&mut self, ui: &mut Ui, allow_finger_labels: bool) {
        use crate::widgets::kit;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Colors").color(theme::SOFT_INK));
            kit::segmented(
                ui,
                &mut self.style.color_mode,
                &[
                    (ColorMode::Finger, "Fingers"),
                    (ColorMode::Role, "Note jobs"),
                ],
            );
            ui.add_space(14.0);
            ui.label(RichText::new("Labels").color(theme::SOFT_INK));
            if !allow_finger_labels && self.style.label_mode == LabelMode::Finger {
                self.style.label_mode = LabelMode::Note;
            }
            let mut options = vec![
                (LabelMode::Note, "Note names"),
                (LabelMode::Interval, "Intervals"),
            ];
            if allow_finger_labels {
                options.insert(0, (LabelMode::Finger, "Finger #"));
            }
            kit::segmented(ui, &mut self.style.label_mode, &options);
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            kit::toggle(ui, &mut self.style.left_handed, "Left-handed");
            ui.add_space(20.0);
            kit::toggle(ui, &mut self.style.show_staff, "Standard notation");
        });
    }

    /// The left sidebar: logo, one row per page, and the player chip at the bottom.
    fn sidebar(&mut self, ui: &mut Ui) {
        use crate::widgets::icons::Icon;
        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(238.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 14,
                        top: 22,
                        bottom: 16,
                    }),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("FretStudy")
                            .font(theme::logo(34.0))
                            .color(theme::ACCENT),
                    );
                });
                ui.add_space(22.0);
                let on_path = matches!(
                    self.screen,
                    Screen::Path | Screen::Lesson | Screen::Welcome | Screen::Placement
                );
                let items = [
                    (Screen::Path, Icon::Map, "My Path", on_path),
                    (
                        Screen::Practice,
                        Icon::Stopwatch,
                        "Practice",
                        self.screen == Screen::Practice,
                    ),
                    (
                        Screen::Songs,
                        Icon::Music,
                        "Songs",
                        self.screen == Screen::Songs,
                    ),
                    (
                        Screen::Chords,
                        Icon::Guitar,
                        "Chords",
                        self.screen == Screen::Chords,
                    ),
                    (
                        Screen::Neck,
                        Icon::Search,
                        "Neck Explorer",
                        self.screen == Screen::Neck,
                    ),
                    (
                        Screen::Tuner,
                        Icon::Target,
                        "Tuner",
                        self.screen == Screen::Tuner,
                    ),
                    (
                        Screen::SoundCheck,
                        Icon::Speaker,
                        "Sound Check",
                        self.screen == Screen::SoundCheck,
                    ),
                    (
                        Screen::Settings,
                        Icon::Gear,
                        "Settings",
                        self.screen == Screen::Settings,
                    ),
                ];
                for (screen, icon, label, active) in items {
                    if nav_item(ui, icon, label, active).clicked() {
                        self.screen = match screen {
                            Screen::Path if self.lesson.is_some() => Screen::Lesson,
                            Screen::Path
                                if !self.profile.as_ref().is_some_and(|p| p.placement_done) =>
                            {
                                Screen::Welcome
                            }
                            other => other,
                        };
                    }
                    ui.add_space(4.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if let Some(p) = self.profile.clone()
                        && profile_chip(ui, &p.name).clicked()
                    {
                        self.screen = Screen::Settings;
                    }
                    ui.add_space(6.0);
                    ui.separator();
                });
            });
    }
}

/// One sidebar row: icon and label; the current page is orange with a bar on the left.
fn nav_item(
    ui: &mut Ui,
    icon: crate::widgets::icons::Icon,
    label: &str,
    active: bool,
) -> egui::Response {
    use egui::{Align2, CornerRadius, FontId, Rect, Sense, Vec2};
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label)
    });
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::same(12), theme::ACCENT_SOFT);
        p.rect_filled(
            Rect::from_min_size(
                rect.left_top() + Vec2::new(0.0, 8.0),
                Vec2::new(4.0, rect.height() - 16.0),
            ),
            CornerRadius::same(2),
            theme::ACCENT,
        );
    } else if resp.hovered() {
        p.rect_filled(
            rect,
            CornerRadius::same(12),
            egui::Color32::from_rgb(0xF7, 0xF0, 0xE7),
        );
    }
    let ink = if active { theme::ACCENT } else { theme::INK };
    crate::widgets::icons::paint(
        p,
        Rect::from_center_size(rect.left_center() + Vec2::new(30.0, 0.0), Vec2::splat(25.0)),
        icon,
        ink,
    );
    p.text(
        rect.left_center() + Vec2::new(60.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        if active {
            theme::heading(18.0)
        } else {
            FontId::proportional(18.0)
        },
        ink,
    );
    resp
}

/// The player at the bottom of the sidebar: initials in a circle, name, and a chevron.
fn profile_chip(ui: &mut Ui, name: &str) -> egui::Response {
    use egui::{Align2, FontId, Sense, Vec2};
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{name}: settings and players"),
        )
    });
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(
            rect,
            egui::CornerRadius::same(12),
            egui::Color32::from_rgb(0xF7, 0xF0, 0xE7),
        );
    }
    let initials: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let c = rect.left_center() + Vec2::new(28.0, 0.0);
    p.circle_filled(c, 21.0, theme::ACCENT);
    p.text(
        c,
        Align2::CENTER_CENTER,
        initials,
        theme::heading(16.0),
        egui::Color32::WHITE,
    );
    let galley = p.layout(
        name.to_string(),
        FontId::proportional(16.0),
        theme::INK,
        rect.width() - 100.0,
    );
    p.galley(
        egui::Pos2::new(rect.left() + 58.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::INK,
    );
    crate::widgets::icons::paint(
        p,
        egui::Rect::from_center_size(
            rect.right_center() - Vec2::new(16.0, 0.0),
            Vec2::splat(14.0),
        ),
        crate::widgets::icons::Icon::ChevronRight,
        theme::INK,
    );
    resp
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // Pull microphone audio once per frame and hand it to whoever is listening.
        self.mic.poll();
        // Lessons in another tuning (Drop D, Open G…) retune everything while they're open.
        let wanted = match (&self.screen, &self.lesson) {
            (Screen::Lesson, Some(l)) => content::tuning_named(l.lesson.tuning.as_deref()),
            _ => Tuning::standard(),
        };
        if wanted != self.tuning {
            self.tuning = wanted;
        }
        if let Some(tuner) = self.tuner.as_mut() {
            tuner.target_string = self.tuner_target;
            tuner.tuning = self.tuning.clone();
            tuner.push(&self.mic.frame_samples);
        }
        self.calibration.update(&self.sound, &self.mic);
        if let Some(Ok(ms)) = self.calibration.result {
            self.latency_ms = Some(ms);
        }

        if self.profile.is_none() {
            self.screen = Screen::Profiles;
        } else {
            self.sidebar(ui);
        }

        let errors: Vec<String> = [&self.sound.error, &self.store_error]
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        let central = egui::Frame::new()
            .fill(theme::BG)
            .inner_margin(egui::Margin {
                left: 30,
                right: 30,
                top: 18,
                bottom: 16,
            });
        egui::CentralPanel::default().frame(central).show(ui, |ui| {
            for err in &errors {
                ui.colored_label(theme::VERMILLION, err);
            }
            match self.screen {
                Screen::Profiles => screens::profiles::show(self, ui),
                Screen::Welcome => screens::welcome::show(self, ui),
                Screen::Placement => screens::placement::show(self, ui),
                Screen::Path => screens::path::show(self, ui),
                Screen::Lesson => {
                    if let Some(mut runner) = self.lesson.take() {
                        let keep = runner.show(self, ui);
                        if keep && !self.close_lesson {
                            self.lesson = Some(runner);
                        } else {
                            self.screen = Screen::Path;
                        }
                    } else {
                        self.screen = Screen::Path;
                    }
                    self.close_lesson = false;
                }
                Screen::Practice => screens::practice::show(self, ui),
                Screen::Songs => screens::songs::show(self, ui),
                Screen::Chords => screens::chords::show(self, ui),
                Screen::Neck => screens::neck::show(self, ui),
                Screen::Tuner => screens::tuner::show(self, ui),
                Screen::SoundCheck => screens::sound_check::show(self, ui),
                Screen::Settings => screens::settings::show(self, ui),
            }
        });

        if let Some(id) = self.pending_open.take() {
            self.open_lesson(&id);
        }
        self.sync_settings(ui.ctx());

        if self.mic.is_on() || self.sound.animating() || self.calibration.running {
            ui.ctx().request_repaint();
        }
    }
}
