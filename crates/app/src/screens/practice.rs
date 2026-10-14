//! Practice: Today's Session, spaced review of finished lessons, a metronome,
//! One-Minute Changes with any chords you know, and the ear gym.

use super::card;
use crate::app::{App, Screen};
use crate::lesson::activities;
use crate::lesson::quiz::{Quiz, QuizOutcome, shuffled};
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit;
use crate::widgets::rhythm;
use content::{Activity, EarKind, Step};
use egui::{RichText, Ui};
use theory::library;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Today,
    Review,
    Metronome,
    Changes,
    Ear,
}

struct ReviewRun {
    lesson: String,
    title: String,
    quiz: Quiz,
    result: Option<bool>,
}

pub struct PracticeUi {
    pub tab: Tab,
    bpm: f32,
    per_bar: usize,
    chord_a: String,
    chord_b: String,
    changes: Option<(Activity, Box<activities::State>)>,
    ear: Option<(Activity, Box<activities::State>)>,
    review: Option<ReviewRun>,
}

impl Default for PracticeUi {
    fn default() -> Self {
        PracticeUi {
            tab: Tab::Today,
            bpm: 80.0,
            per_bar: 4,
            chord_a: "G".into(),
            chord_b: "C".into(),
            changes: None,
            ear: None,
            review: None,
        }
    }
}

/// Every ear game, with what it's called here.
const EAR_GAMES: [(EarKind, &str); 8] = [
    (EarKind::HighLow, "Higher or lower?"),
    (EarKind::MajorMinor, "Major or minor?"),
    (EarKind::HomeOrAway, "Home or away?"),
    (EarKind::FourOrFive, "The 4 chord or the 5 chord?"),
    (EarKind::Thirds, "Major 3rd or minor 3rd?"),
    (EarKind::FourthsFifths, "4th or 5th?"),
    (EarKind::Progressions, "Name that progression"),
    (EarKind::StraightOrSwing, "Straight or swing?"),
];

pub fn show(app: &mut App, ui: &mut Ui) {
    let mut ui_state = std::mem::take(&mut app.practice);
    let before = ui_state.tab;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        kit::page_header(
            ui,
            "Practice",
            "Practice makes progress",
            "Short, focused practice every day: reviews, a metronome, chord changes, and ear games.",
        );
        let due = app.due_reviews().len();
        let review = if due > 0 { format!("Review ({due})") } else { "Review".to_string() };
        kit::segmented(
            ui,
            &mut ui_state.tab,
            &[
                (Tab::Today, "Today's Session"),
                (Tab::Review, review.as_str()),
                (Tab::Metronome, "Metronome"),
                (Tab::Changes, "One-Minute Changes"),
                (Tab::Ear, "Ear Gym"),
            ],
        );
        ui.add_space(12.0);
        if ui_state.tab != before {
            // Leaving a tab stops whatever it was playing.
            app.sound.cancel_scheduled();
            for (_, s) in [&mut ui_state.changes, &mut ui_state.ear].into_iter().flatten() {
                s.leave(app);
            }
        }
        match ui_state.tab {
            Tab::Today => today(app, ui, &mut ui_state),
            Tab::Review => review_tab(app, ui, &mut ui_state),
            Tab::Metronome => card(ui, |ui| metronome(app, ui, &mut ui_state)),
            Tab::Changes => card(ui, |ui| changes(app, ui, &mut ui_state)),
            Tab::Ear => card(ui, |ui| ear(app, ui, &mut ui_state)),
        }
    });
    app.practice = ui_state;
}

fn today(app: &mut App, ui: &mut Ui, st: &mut PracticeUi) {
    ui.label(RichText::new("Today's Session").font(theme::heading(24.0)));
    ui.label(
        RichText::new(
            "A little every day beats a lot once a week. Work down the list — about 15 minutes.",
        )
        .size(17.0)
        .color(theme::SOFT_INK),
    );
    ui.add_space(8.0);
    let due = app.due_reviews().len();
    let next = app
        .curriculum
        .next_lesson(&app.progress)
        .and_then(|id| app.curriculum.lesson(id))
        .map(|l| (l.id.clone(), l.title.clone()));
    let rows: Vec<(&str, String, &str)> = vec![
        ("🎯", "Tune up (1 minute)".into(), "tune"),
        (
            "🔁",
            if due > 0 {
                format!(
                    "Review {due} lesson{} (3 minutes)",
                    if due == 1 { "" } else { "s" }
                )
            } else {
                "No reviews due today — nice!".into()
            },
            "review",
        ),
        (
            "🎸",
            "One-Minute Changes on your hardest pair (2 minutes)".into(),
            "changes",
        ),
        (
            "📘",
            match &next {
                Some((id, title)) => format!("Next lesson: {id} {title} (5–8 minutes)"),
                None => "Every lesson so far is done!".into(),
            },
            "lesson",
        ),
        ("👂", "Ear gym: one game (2 minutes)".into(), "ear"),
    ];
    for (icon, text, what) in rows {
        card(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(RichText::new(icon).size(24.0));
                ui.label(RichText::new(&text).size(18.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let enabled = match what {
                        "review" => due > 0,
                        "lesson" => next.is_some(),
                        _ => true,
                    };
                    if ui
                        .add_enabled_ui(enabled, |ui| kit::primary(ui, Some(Icon::Play), "Go"))
                        .inner
                        .clicked()
                    {
                        match what {
                            "tune" => app.screen = Screen::Tuner,
                            "review" => st.tab = Tab::Review,
                            "changes" => st.tab = Tab::Changes,
                            "ear" => st.tab = Tab::Ear,
                            _ => {
                                if let Some((id, _)) = &next {
                                    app.pending_open = Some(id.clone());
                                }
                            }
                        }
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
    ui.add_space(10.0);
    streak(app, ui);
}

/// Minutes practiced on each of the last 7 days that had practice.
fn streak(app: &App, ui: &mut Ui) {
    let days = app.practice_days(7);
    if days.is_empty() {
        return;
    }
    ui.label(RichText::new("Your recent practice").font(theme::heading(20.0)));
    let max = days.iter().map(|d| d.1).max().unwrap_or(1).max(60) as f32;
    ui.horizontal(|ui| {
        for (day, secs) in days {
            ui.vertical(|ui| {
                let h = 80.0 * secs as f32 / max;
                let (r, painter) =
                    ui.allocate_painter(egui::vec2(44.0, 84.0), egui::Sense::hover());
                let bar = egui::Rect::from_min_max(
                    egui::pos2(r.rect.left() + 8.0, r.rect.bottom() - h.max(3.0)),
                    egui::pos2(r.rect.right() - 8.0, r.rect.bottom()),
                );
                painter.rect_filled(bar, egui::CornerRadius::same(4), theme::GREEN);
                ui.label(RichText::new(format!("{}m", secs / 60)).small());
                ui.label(
                    RichText::new(day.get(5..).unwrap_or(&day))
                        .small()
                        .color(theme::SOFT_INK),
                );
            });
        }
    });
}

fn review_tab(app: &mut App, ui: &mut Ui, st: &mut PracticeUi) {
    ui.label(RichText::new("Review").font(theme::heading(24.0)));
    ui.label(
        RichText::new(
            "Lessons come back after 1 day, then 3, 7, and 21 days. A quick quiz keeps them fresh.",
        )
        .size(17.0)
        .color(theme::SOFT_INK),
    );
    ui.add_space(8.0);
    if let Some(run) = st.review.as_mut() {
        ui.label(RichText::new(format!("{} {}", run.lesson, run.title)).font(theme::heading(22.0)));
        if let Some(outcome) = run.quiz.show(app, ui) {
            let passed = matches!(outcome, QuizOutcome::Passed(_));
            app.record_review(&run.lesson, passed);
            run.result = Some(passed);
        }
        if let Some(passed) = run.result {
            ui.label(
                RichText::new(if passed {
                    "✔ Still got it! This lesson will come back later."
                } else {
                    "It'll come back tomorrow. You can open the lesson for a refresher."
                })
                .size(18.0)
                .color(if passed { theme::GREEN } else { theme::ORANGE }),
            );
            let mut next = false;
            ui.horizontal(|ui| {
                if !passed && ui.button("📘 Open the lesson").clicked() {
                    app.pending_open = Some(run.lesson.clone());
                }
                next = ui.button(RichText::new("Next ▶").size(18.0)).clicked();
            });
            if next {
                st.review = None;
            }
        }
        return;
    }
    let due = app.due_reviews();
    if due.is_empty() {
        ui.label(
            RichText::new("🎉 Nothing to review today. Finished lessons will show up here.")
                .size(18.0),
        );
        return;
    }
    for (id, stage) in due {
        let Some(lesson) = app.curriculum.lesson(&id).cloned() else {
            continue;
        };
        card(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{} {}", lesson.id, lesson.title)).size(18.0));
                ui.label(
                    RichText::new(format!("round {}", stage + 1))
                        .small()
                        .color(theme::SOFT_INK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("Review ▶").size(17.0)).clicked() {
                        st.review = review_quiz(&lesson);
                        if st.review.is_none() {
                            // Nothing to quiz on: count it as reviewed.
                            app.record_review(&lesson.id, true);
                        }
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
}

/// Up to three Check It questions from the lesson, a different mix each day.
fn review_quiz(lesson: &content::Lesson) -> Option<ReviewRun> {
    let questions: Vec<_> = lesson
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Check { questions, .. } => Some(questions.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    if questions.is_empty() {
        return None;
    }
    let order = shuffled(questions.len(), store::day_number() as u64);
    let picked: Vec<_> = order
        .iter()
        .take(3)
        .map(|i| questions[*i].clone())
        .collect();
    Some(ReviewRun {
        lesson: lesson.id.clone(),
        title: lesson.title.clone(),
        quiz: Quiz::new(picked, 66),
        result: None,
    })
}

fn metronome(app: &mut App, ui: &mut Ui, st: &mut PracticeUi) {
    ui.label(RichText::new("Metronome").font(theme::heading(24.0)));
    ui.label(
        RichText::new("Start slow and clean. When it feels easy three times in a row, add 5 BPM.")
            .size(17.0)
            .color(theme::SOFT_INK),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Speed").size(17.0));
        ui.add(
            egui::Slider::new(&mut st.bpm, 40.0..=200.0)
                .step_by(1.0)
                .suffix(" BPM"),
        );
        for d in [-5.0, 5.0] {
            if ui.button(format!("{d:+}")).clicked() {
                st.bpm = (st.bpm + d).clamp(40.0, 200.0);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Beats in a bar").size(17.0));
        for n in [2usize, 3, 4, 6] {
            ui.selectable_value(&mut st.per_bar, n, n.to_string());
        }
    });
    let (beat, flash) = app.sound.metronome_beat().unzip();
    rhythm::beat_pulse(
        ui,
        st.per_bar,
        beat,
        flash.unwrap_or(0.0),
        60.0 * st.per_bar as f32,
    );
    ui.horizontal(|ui| {
        if ui.button(RichText::new("▶ Start").size(18.0)).clicked() {
            app.sound.cancel_scheduled();
            // Five minutes of clicks.
            let beats = (st.bpm * 5.0) as usize;
            app.sound.start_metronome_in(st.bpm, beats, st.per_bar);
        }
        if ui.button(RichText::new("■ Stop").size(18.0)).clicked() {
            app.sound.cancel_scheduled();
            app.sound.metronome.clear();
        }
    });
    if beat.is_some() {
        ui.ctx().request_repaint();
    }
}

/// Library chords whose lesson is done (completed or placed out), in teaching order.
fn known_chords(app: &App) -> Vec<&'static library::ChordEntry> {
    library::CHORDS
        .iter()
        .filter(|e| app.progress.get(e.lesson).is_some_and(|d| d.satisfies()))
        .collect()
}

fn changes(app: &mut App, ui: &mut Ui, st: &mut PracticeUi) {
    ui.label(RichText::new("One-Minute Changes").font(theme::heading(24.0)));
    let known = known_chords(app);
    if known.len() < 2 {
        ui.label(RichText::new("Finish a couple of chord lessons first — then any two chords you know show up here.").size(17.0));
        return;
    }
    let label_of = |id: &str| library::find(id).map(|e| e.label).unwrap_or("?");
    let mut changed = false;
    ui.horizontal(|ui| {
        for (which, slot) in [
            ("First chord", &mut st.chord_a),
            ("Second chord", &mut st.chord_b),
        ] {
            ui.label(RichText::new(which).size(17.0));
            egui::ComboBox::from_id_salt(which)
                .selected_text(label_of(slot))
                .show_ui(ui, |ui| {
                    for e in &known {
                        if ui
                            .selectable_label(slot.as_str() == e.id, e.label)
                            .clicked()
                        {
                            *slot = e.id.to_string();
                            changed = true;
                        }
                    }
                });
            ui.add_space(12.0);
        }
    });
    if !known.iter().any(|e| e.id == st.chord_a) || !known.iter().any(|e| e.id == st.chord_b) {
        st.chord_a = known[0].id.to_string();
        st.chord_b = known[1].id.to_string();
        changed = true;
    }
    if changed {
        if let Some((_, s)) = st.changes.as_mut() {
            s.leave(app);
        }
        st.changes = None;
    }
    let (activity, state) = st.changes.get_or_insert_with(|| {
        let a = Activity::ChangeTrainer {
            a: st.chord_a.clone(),
            b: st.chord_b.clone(),
            target: 30,
        };
        let s = Box::new(activities::State::new(&a));
        (a, s)
    });
    ui.add_space(8.0);
    activities::show(app, ui, activity, state);
}

/// Ear games unlock when a finished lesson uses them.
fn unlocked_ear_games(app: &App) -> Vec<EarKind> {
    let mut kinds = Vec::new();
    for lesson in app.curriculum.lessons() {
        if !app.progress.get(&lesson.id).is_some_and(|d| d.satisfies()) {
            continue;
        }
        for step in &lesson.steps {
            if let Step::Activity {
                activity: Activity::EarQuiz { kind, .. },
                ..
            } = step
                && !kinds.contains(kind)
            {
                kinds.push(*kind);
            }
        }
    }
    kinds
}

fn ear(app: &mut App, ui: &mut Ui, st: &mut PracticeUi) {
    ui.label(RichText::new("Ear Gym").font(theme::heading(24.0)));
    let unlocked = unlocked_ear_games(app);
    ui.horizontal_wrapped(|ui| {
        for (kind, name) in EAR_GAMES {
            let open = unlocked.contains(&kind);
            let current = st
                .ear
                .as_ref()
                .is_some_and(|(a, _)| matches!(a, Activity::EarQuiz { kind: k, .. } if *k == kind));
            let text = if open {
                RichText::new(name).size(17.0)
            } else {
                RichText::new(format!("🔒 {name}"))
                    .size(17.0)
                    .color(theme::SOFT_INK)
            };
            let resp = ui.add_enabled(open, egui::Button::selectable(current, text));
            if resp.clicked() {
                let a = Activity::EarQuiz { kind, rounds: 10 };
                let s = Box::new(activities::State::new(&a));
                st.ear = Some((a, s));
            }
        }
    });
    if unlocked.is_empty() {
        ui.label(RichText::new("Ear games unlock as you finish lessons that use them.").size(17.0));
        return;
    }
    ui.add_space(8.0);
    if let Some((activity, state)) = st.ear.as_mut() {
        activities::show(app, ui, activity, state);
    } else {
        ui.label(RichText::new("Pick a game.").color(theme::SOFT_INK));
    }
}
