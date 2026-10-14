//! "My Path": what to do next, overall progress, and every level's lessons as tiles.

use crate::app::App;
use crate::theme;
use crate::widgets::chord_box;
use crate::widgets::icons::{self, Icon};
use crate::widgets::kit::{self, card, card_tinted};
use content::{Lesson, Status, Step, Thread, Visual};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use theory::library;

pub fn thread_icon(t: Thread) -> Icon {
    match t {
        Thread::Technique => Icon::Hand,
        Thread::Chords => Icon::Grid,
        Thread::Rhythm => Icon::Metronome,
        Thread::Notes => Icon::Music,
        Thread::TheoryEar => Icon::Ear,
        Thread::Song => Icon::Star,
    }
}

/// (tile fill, badge color) for a lesson's status.
fn status_look(status: Status) -> (Color32, Color32) {
    match status {
        Status::Completed => (theme::GREEN_SOFT, theme::GREEN),
        Status::PlacedOut => (theme::BLUE_SOFT, theme::BLUE),
        Status::InProgress => (theme::YELLOW_SOFT, theme::ORANGE),
        Status::Ready => (Color32::WHITE, theme::ACCENT),
        Status::Locked | Status::ComingSoon => {
            (Color32::from_rgb(0xF6, 0xF2, 0xEC), theme::LIGHT_GRAY)
        }
    }
}

/// The first chords a lesson shows (for the little diagrams on the "Continue" card).
fn lesson_chords(lesson: &Lesson) -> Vec<&'static library::ChordEntry> {
    let mut ids = Vec::new();
    for step in &lesson.steps {
        if let Step::Learn {
            visual: Some(v), ..
        } = step
        {
            match v {
                Visual::Chord(a) => ids.push(a.clone()),
                Visual::ChordPair(a, b) => {
                    ids.push(a.clone());
                    ids.push(b.clone());
                }
                _ => {}
            }
        }
        if ids.len() >= 2 {
            break;
        }
    }
    ids.iter()
        .take(2)
        .filter_map(|id| library::find(id))
        .collect()
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let curriculum = app.curriculum.clone();
    let progress = app.progress.clone();
    let next = curriculum.next_lesson(&progress).map(str::to_string);
    let core: Vec<&Lesson> = curriculum
        .levels
        .iter()
        .filter(|l| l.track.is_none())
        .flat_map(|l| l.lessons.iter())
        .collect();
    let done = core
        .iter()
        .filter(|l| progress.get(&l.id).is_some_and(|d| d.satisfies()))
        .count();

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let (title, sub) = if done == 0 {
            ("Let's get started!", "Your path starts here. Each lesson builds on the last, and you can come back to any of them anytime.")
        } else {
            ("Keep going, you're doing great!", "Follow your learning path step by step, and come back anytime to practice with the tools.")
        };
        kit::page_header(ui, "My path", title, sub);

        ui.horizontal_top(|ui| {
            let full = ui.available_width();
            let gap = ui.spacing().item_spacing.x;
            let right_w = 460.0f32.min(full * 0.42);
            let left_w = full - right_w - gap - 1.0;
            ui.allocate_ui(Vec2::new(left_w, 0.0), |ui| continue_card(app, ui, next.as_deref(), left_w));
            ui.allocate_ui(Vec2::new(right_w, 0.0), |ui| {
                card(ui, |ui| {
                    ui.set_width(right_w - 42.0);
                    ui.label(RichText::new("Overall progress").font(theme::heading(22.0)));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let frac = done as f32 / core.len().max(1) as f32;
                        kit::progress_ring(ui, frac, 140.0, &done.to_string(), &format!("of {} lessons", core.len()));
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            stat(ui, Icon::Check, theme::GREEN, &done.to_string(), "Lessons completed");
                            stat(ui, Icon::Book, theme::INK, &(core.len() - done).to_string(), "Lessons remaining");
                            stat(ui, Icon::Lines, theme::INK, &format!("{:.0}%", frac * 100.0), "Overall progress");
                        });
                    });
                });
            });
        });
        notices(app, ui);
        ui.add_space(14.0);

        let next_level = next.as_deref().and_then(|id| curriculum.level_of(id));
        let mut tracks_started = false;
        for level in &curriculum.levels {
            if level.track.is_some() && !tracks_started {
                tracks_started = true;
                ui.add_space(8.0);
                ui.label(RichText::new("Level 8 · Style Tracks").font(theme::heading(28.0)));
                kit::subtitle(ui, "Pick any track you like, in any order. Each one ends with a capstone song. Tone & Gear is open any time after Level 2.");
                ui.add_space(8.0);
            }
            let (ldone, total) = curriculum.level_progress(level.number, &progress);
            let written = level.lessons.iter().any(|l| !l.coming_soon);
            let open_default = if level.track.is_some() {
                next_level == Some(level.number) || (ldone > 0 && ldone < total)
            } else {
                written || next_level == Some(level.number) || ldone > 0
            };
            let open_id = ui.id().with(("level_open", level.number));
            let mut open: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(open_default);
            card(ui, |ui| {
                // Header row: status, title, count, progress, open/close.
                let header = ui.horizontal(|ui| {
                    let frac = ldone as f32 / total.max(1) as f32;
                    if ldone == total && total > 0 {
                        kit::status_dot(ui, true, theme::GREEN, 38.0);
                    } else {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::hover());
                        level_ring(ui.painter(), r, frac);
                    }
                    let name = match &level.track {
                        Some(_) => level.title.clone(),
                        None => format!("Level {} · {}", level.number, level.title),
                    };
                    ui.label(RichText::new(name).font(theme::heading(23.0)));
                    ui.label(RichText::new(format!("{ldone} of {total} done")).size(15.5).color(theme::SOFT_INK));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let chev = if open { Icon::ChevronUp } else { Icon::ChevronDown };
                        icons::show(ui, chev, 18.0, theme::INK);
                        ui.label(RichText::new(format!("{:.0}%", frac * 100.0)).size(16.0));
                        let color = if ldone == total { theme::GREEN } else { theme::SKY_BLUE };
                        kit::progress_bar(ui, frac, color, 190.0);
                    });
                });
                let header_resp = ui.interact(header.response.rect, open_id.with("hdr"), Sense::click());
                if header_resp.clicked() {
                    open = !open;
                }
                ui.label(RichText::new(format!("{} · {}", level.goal, level.weeks)).size(15.0).color(theme::SOFT_INK));
                if !written {
                    ui.label(RichText::new("These lessons are being written. They're listed so you can see where the path goes.").small().color(theme::SOFT_INK));
                }
                if open {
                    ui.add_space(8.0);
                    lesson_grid(app, ui, &level.lessons, next.as_deref());
                }
            });
            ui.data_mut(|d| d.insert_temp(open_id, open));
            ui.add_space(14.0);
        }
    });
}

fn stat(ui: &mut Ui, icon: Icon, color: Color32, value: &str, label: &str) {
    ui.horizontal(|ui| {
        if icon == Icon::Check {
            kit::status_dot(ui, true, color, 26.0);
        } else {
            icons::show(ui, icon, 24.0, color);
        }
        ui.add_space(6.0);
        ui.label(RichText::new(value).font(theme::heading(20.0)));
        ui.label(RichText::new(label).size(15.5).color(theme::SOFT_INK));
    });
    ui.add_space(6.0);
}

/// A small ring showing how much of a level is done.
fn level_ring(p: &egui::Painter, r: Rect, frac: f32) {
    let c = r.center();
    let rad = r.width() / 2.0 - 4.0;
    p.circle_stroke(
        c,
        rad,
        Stroke::new(5.0, Color32::from_rgb(0xEC, 0xE6, 0xDD)),
    );
    if frac > 0.0 {
        let start = -std::f32::consts::FRAC_PI_2;
        let n = 40;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = start + std::f32::consts::TAU * frac * i as f32 / n as f32;
                Pos2::new(c.x + rad * a.cos(), c.y + rad * a.sin())
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(5.0, theme::SKY_BLUE)));
    }
}

fn continue_card(app: &mut App, ui: &mut Ui, next: Option<&str>, width: f32) {
    let curriculum = app.curriculum.clone();
    card_tinted(ui, Color32::from_rgb(0xFF, 0xF3, 0xE9), |ui| {
        ui.set_width(width - 42.0);
        let Some(lesson) = next.and_then(|id| curriculum.lesson(id)) else {
            ui.label(RichText::new("🎉 You've finished every lesson!").font(theme::heading(28.0)));
            kit::subtitle(
                ui,
                "Keep practicing with Chords, Neck Explorer, Songs, and the Tuner, and review any lesson anytime.",
            );
            return;
        };
        ui.horizontal_top(|ui| {
            let chords = lesson_chords(lesson);
            let diagrams_w = chords.len() as f32 * 118.0;
            ui.vertical(|ui| {
                ui.set_width((width - 42.0 - diagrams_w - 10.0).max(260.0));
                let started = app.progress.get(&lesson.id) == Some(&content::Done::InProgress);
                ui.label(
                    RichText::new(if started {
                        "CONTINUE LEARNING"
                    } else {
                        "NEXT UP"
                    })
                    .size(14.0)
                    .strong()
                    .color(theme::ACCENT),
                );
                ui.label(
                    RichText::new(format!("{}   {}", lesson.id, lesson.title))
                        .font(theme::heading(28.0)),
                );
                ui.label(
                    RichText::new(&lesson.goal)
                        .size(17.0)
                        .color(theme::SOFT_INK),
                );
                ui.label(
                    RichText::new(format!(
                        "{} · about {} minutes",
                        lesson.thread.name(),
                        lesson.minutes
                    ))
                    .size(14.5)
                    .color(theme::SOFT_INK),
                );
                ui.add_space(10.0);
                let label = if started {
                    "Continue lesson"
                } else {
                    "Start lesson"
                };
                if kit::primary(ui, Some(Icon::Play), label).clicked() {
                    app.pending_open = Some(lesson.id.clone());
                }
            });
            for e in chords {
                chord_box::mini(ui, &e.shape(), &theme::glyphs(&e.chord().pretty()), 104.0);
                ui.add_space(6.0);
            }
        });
    });
}

fn notices(app: &App, ui: &mut Ui) {
    if let (Some(store), Some(p)) = (&app.store, &app.profile)
        && let Ok(recent) = store.recent_checks(p.id, 2)
        && recent.len() == 2
        && recent.iter().all(|(_, passed)| !passed)
    {
        ui.add_space(8.0);
        kit::tip(
            ui,
            "Stuck on a check? That's normal! Try reviewing an earlier lesson (tap any finished tile), then come back.",
        );
    }
}

/// Lesson tiles, four to a row.
fn lesson_grid(app: &mut App, ui: &mut Ui, lessons: &[Lesson], next: Option<&str>) {
    let curriculum = app.curriculum.clone();
    let progress = app.progress.clone();
    let gap = 12.0;
    let cols = if ui.available_width() > 900.0 { 4 } else { 3 };
    let tile_w = (ui.available_width() - gap * (cols - 1) as f32) / cols as f32;
    for row in lessons.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for lesson in row {
                let status = curriculum.status(&lesson.id, &progress);
                let (fill, badge) = status_look(status);
                let is_next = next == Some(lesson.id.as_str());
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(tile_w, 96.0), Sense::click());
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("{} {}", lesson.id, lesson.title),
                    )
                });
                let p = ui.painter();
                let border = if is_next {
                    Stroke::new(2.5, theme::ACCENT)
                } else if resp.hovered() {
                    Stroke::new(1.0, Color32::from_rgb(0xF2, 0xB8, 0x95))
                } else {
                    Stroke::new(1.0, theme::BORDER)
                };
                p.rect(
                    rect,
                    CornerRadius::same(12),
                    fill,
                    border,
                    egui::StrokeKind::Inside,
                );
                // Status badge and id.
                let badge_c = rect.left_top() + Vec2::new(22.0, 22.0);
                match status {
                    Status::Completed | Status::PlacedOut => {
                        p.circle_filled(badge_c, 10.0, badge);
                        icons::paint(
                            p,
                            Rect::from_center_size(badge_c, Vec2::splat(11.0)),
                            Icon::Check,
                            Color32::WHITE,
                        );
                    }
                    Status::Locked | Status::ComingSoon => {
                        icons::paint(
                            p,
                            Rect::from_center_size(badge_c, Vec2::splat(16.0)),
                            Icon::Lock,
                            theme::SOFT_INK,
                        );
                    }
                    Status::InProgress => {
                        p.circle_stroke(badge_c, 9.0, Stroke::new(2.5, badge));
                    }
                    Status::Ready => {
                        icons::paint(
                            p,
                            Rect::from_center_size(badge_c, Vec2::splat(14.0)),
                            Icon::Play,
                            badge,
                        );
                    }
                }
                p.text(
                    badge_c + Vec2::new(18.0, 0.0),
                    Align2::LEFT_CENTER,
                    &lesson.id,
                    FontId::proportional(14.0),
                    theme::SOFT_INK,
                );
                // Thread icon and title.
                let muted = matches!(status, Status::Locked | Status::ComingSoon);
                let ink = if muted { theme::SOFT_INK } else { theme::INK };
                let galley = p.layout(
                    lesson.title.clone(),
                    FontId::proportional(15.0),
                    ink,
                    tile_w - 84.0,
                );
                let lines = galley.rows.len().max(1) as f32;
                let title_top = rect.top() + 42.0 + if lines < 2.0 { 9.0 } else { 0.0 };
                icons::paint(
                    p,
                    Rect::from_center_size(
                        Pos2::new(rect.left() + 26.0, title_top + 9.0),
                        Vec2::splat(22.0),
                    ),
                    thread_icon(lesson.thread),
                    ink,
                );
                p.galley(Pos2::new(rect.left() + 48.0, title_top), galley, ink);
                if !muted {
                    icons::paint(
                        p,
                        Rect::from_center_size(
                            rect.right_center() - Vec2::new(18.0, 0.0),
                            Vec2::splat(14.0),
                        ),
                        Icon::ChevronRight,
                        theme::INK,
                    );
                }
                let tip = match status {
                    Status::Locked => {
                        let missing: Vec<&str> = lesson
                            .prerequisites
                            .iter()
                            .filter(|p| !progress.get(*p).is_some_and(|d| d.satisfies()))
                            .map(String::as_str)
                            .collect();
                        format!("Finish {} first.", missing.join(", "))
                    }
                    Status::ComingSoon => "Coming soon.".to_string(),
                    Status::PlacedOut => {
                        "You already know this (from the placement quiz). Click to review."
                            .to_string()
                    }
                    Status::Completed => "Done! Click to review.".to_string(),
                    _ => format!(
                        "{} · about {} minutes",
                        lesson.thread.name(),
                        lesson.minutes
                    ),
                };
                let resp = resp.on_hover_text(tip);
                if resp.clicked() && !muted {
                    app.pending_open = Some(lesson.id.clone());
                }
            }
        });
        ui.add_space(gap - 8.0);
    }
}
