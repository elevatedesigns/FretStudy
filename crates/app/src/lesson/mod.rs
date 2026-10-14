//! The lesson player: Learn → Try → Check It, one step at a time.

pub mod activities;
pub mod quiz;
pub mod visuals;

use crate::app::{App, Screen};
use crate::screens::card;
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit;
use content::{Lesson, Step};
use egui::{RichText, Ui};
use quiz::{Quiz, QuizOutcome};
use std::time::Instant;

enum StepState {
    Learn,
    Activity(Box<activities::State>),
    Check(Quiz),
}

pub struct LessonRunner {
    pub lesson: Lesson,
    step: usize,
    states: Vec<StepState>,
    started: Instant,
    /// Set when Check It is passed: the score.
    completed: Option<f32>,
}

impl LessonRunner {
    pub fn new(lesson: Lesson) -> LessonRunner {
        let states = lesson
            .steps
            .iter()
            .map(|s| match s {
                Step::Learn { .. } => StepState::Learn,
                Step::Activity { activity, .. } => {
                    StepState::Activity(Box::new(activities::State::new(activity)))
                }
                Step::Check {
                    questions,
                    pass_percent,
                } => StepState::Check(Quiz::new(questions.clone(), *pass_percent)),
            })
            .collect();
        LessonRunner {
            lesson,
            step: 0,
            states,
            started: Instant::now(),
            completed: None,
        }
    }

    /// Jump straight to a step (used by visual tests).
    #[cfg(test)]
    pub fn jump(&mut self, step: usize) {
        self.step = step.min(self.lesson.steps.len().saturating_sub(1));
    }

    fn go_to(&mut self, app: &mut App, step: usize) {
        if let Some(StepState::Activity(a)) = self.states.get_mut(self.step) {
            a.leave(app);
        }
        self.step = step.min(self.lesson.steps.len().saturating_sub(1));
    }

    /// Draws the lesson. Returns false when the learner leaves it.
    pub fn show(&mut self, app: &mut App, ui: &mut Ui) -> bool {
        let mut keep = true;
        let total = self.lesson.steps.len();

        // Header: back button, title, step pills
        ui.horizontal(|ui| {
            if kit::secondary(ui, Some(Icon::ChevronLeft), "My Path").clicked() {
                if let Some(StepState::Activity(a)) = self.states.get_mut(self.step) {
                    a.leave(app);
                }
                keep = false;
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("{} · {}", self.lesson.id, self.lesson.title))
                    .font(theme::heading(26.0)),
            );
        });
        ui.label(
            RichText::new(&self.lesson.goal)
                .color(theme::SOFT_INK)
                .size(16.0),
        );
        ui.add_space(4.0);
        let mut jump = None;
        ui.horizontal_wrapped(|ui| {
            for (i, s) in self.lesson.steps.iter().enumerate() {
                let kind = match s {
                    Step::Learn { .. } => "📖",
                    Step::Activity { .. } => "🎸",
                    Step::Check { .. } => "✅",
                };
                let text = RichText::new(format!("{kind} {}", s.title())).size(15.0);
                if ui.selectable_label(i == self.step, text).clicked() {
                    jump = Some(i);
                }
            }
        });
        if let Some(i) = jump {
            self.go_to(app, i);
        }
        ui.separator();

        let step = self.lesson.steps[self.step].clone();
        let mut outcome = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 56.0)
            .show(ui, |ui| {
                card(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match (&step, &mut self.states[self.step]) {
                        (
                            Step::Learn {
                                title,
                                explain,
                                more,
                                facts,
                                visual,
                                tips,
                            },
                            StepState::Learn,
                        ) => {
                            ui.label(RichText::new(title).font(theme::heading(24.0)));
                            ui.add_space(4.0);
                            ui.label(RichText::new(theme::glyphs(explain)).size(19.0));
                            if !facts.is_empty() {
                                ui.add_space(6.0);
                                for f in facts {
                                    ui.label(
                                        RichText::new(format!("•  {}", theme::glyphs(f)))
                                            .size(17.0),
                                    );
                                }
                            }
                            if let Some(m) = more {
                                egui::CollapsingHeader::new(
                                    RichText::new("More detail").color(theme::BLUE),
                                )
                                .id_salt(("more", self.step))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(theme::glyphs(m)).size(16.0));
                                });
                            }
                            if let Some(v) = visual {
                                ui.add_space(10.0);
                                visuals::show(app, ui, v, &format!("learn-{}", self.step));
                            }
                            if !tips.is_empty() {
                                ui.add_space(10.0);
                                watch_out(ui, tips);
                            }
                        }
                        (
                            Step::Activity {
                                title,
                                instructions,
                                activity,
                            },
                            StepState::Activity(state),
                        ) => {
                            ui.label(RichText::new(title).font(theme::heading(24.0)));
                            ui.label(RichText::new(theme::glyphs(instructions)).size(18.0));
                            ui.add_space(8.0);
                            activities::show(app, ui, activity, state);
                        }
                        (Step::Check { .. }, StepState::Check(quiz)) => {
                            ui.label(RichText::new("Check It").font(theme::heading(24.0)));
                            if self.completed.is_none() && !self.lesson.practice.is_empty() {
                                practice_plan(ui, &self.lesson.practice);
                                ui.add_space(6.0);
                            }
                            if let Some(score) = self.completed {
                                celebrate(app, ui, &self.lesson, score);
                            } else {
                                outcome = quiz.show(app, ui);
                            }
                        }
                        _ => {}
                    }
                });
            });

        match outcome {
            Some(QuizOutcome::Passed(score)) => {
                self.completed = Some(score);
                app.finish_lesson(
                    &self.lesson.id,
                    score,
                    true,
                    self.started.elapsed().as_secs() as u32,
                );
            }
            Some(QuizOutcome::Failed(score)) => {
                app.finish_lesson(
                    &self.lesson.id,
                    score,
                    false,
                    self.started.elapsed().as_secs() as u32,
                );
            }
            None => {}
        }

        // Footer navigation. Arrow keys also move between steps (when nothing is being typed).
        let typing = ui.ctx().egui_wants_keyboard_input();
        let (left, right) = if typing {
            (false, false)
        } else {
            ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                )
            })
        };
        ui.separator();
        ui.horizontal(|ui| {
            let back = self.step > 0
                && (ui
                    .button(RichText::new("◀ Back").size(18.0))
                    .on_hover_text("Left arrow key")
                    .clicked()
                    || left);
            if back {
                let s = self.step - 1;
                self.go_to(app, s);
                return;
            }
            ui.label(
                RichText::new(format!("Step {} of {}", self.step + 1, total))
                    .color(theme::SOFT_INK),
            );
            if self.step + 1 < total
                && (kit::primary(ui, None, "Next ▶")
                    .on_hover_text("Right arrow key")
                    .clicked()
                    || right)
            {
                let s = self.step + 1;
                self.go_to(app, s);
            }
        });
        keep
    }
}

fn celebrate(app: &mut App, ui: &mut Ui, lesson: &Lesson, score: f32) {
    let playful = app.playful();
    let title = if playful {
        "🎉 Lesson complete! 🎸"
    } else {
        "Lesson complete"
    };
    ui.label(
        RichText::new(title)
            .font(theme::heading(30.0))
            .color(theme::GREEN),
    );
    ui.label(RichText::new(format!("You scored {:.0}% on Check It.", score * 100.0)).size(18.0));
    if !lesson.practice.is_empty() {
        ui.add_space(6.0);
        practice_plan(ui, &lesson.practice);
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let next = app.next_lesson_after(&lesson.id);
        if let Some(next) = next {
            let label = format!(
                "Next lesson: {} ▶",
                app.curriculum
                    .lesson(&next)
                    .map(|l| l.title.clone())
                    .unwrap_or_default()
            );
            if ui
                .button(RichText::new(label).size(18.0).strong())
                .clicked()
            {
                app.pending_open = Some(next);
            }
        }
        if ui
            .button(RichText::new("Back to My Path").size(18.0))
            .clicked()
        {
            app.screen = Screen::Path;
            app.close_lesson = true;
        }
    });
}

/// The "Watch out" box: common mistakes and how to fix them.
fn watch_out(ui: &mut Ui, tips: &[String]) {
    egui::Frame::new()
        .fill(theme::YELLOW_SOFT)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::icons::show(ui, Icon::Bulb, 22.0, theme::ORANGE);
                ui.label(RichText::new("Watch out").font(theme::heading(18.0)));
            });
            for t in tips {
                ui.label(RichText::new(format!("•  {}", theme::glyphs(t))).size(16.5));
            }
        });
}

/// How to practice after the lesson.
fn practice_plan(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::BLUE_SOFT)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::icons::show(ui, Icon::Stopwatch, 22.0, theme::BLUE);
                ui.label(RichText::new("Your practice plan").font(theme::heading(18.0)));
            });
            ui.label(RichText::new(theme::glyphs(text)).size(16.5));
        });
}
