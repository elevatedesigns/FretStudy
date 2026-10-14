//! The placement quiz screen (docs/03-Placement-Assessment.md).

use super::card;
use crate::app::{App, Screen};
use crate::lesson::quiz::{question_card, shuffled};
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit;
use content::placement::{Experience, PlacementResult, Profile, Session, placed_out_lessons};
use egui::{RichText, Ui};

enum Stage {
    Experience,
    Claims(Experience),
    Quiz(Box<Session>),
    /// The result, lessons to place out, and the finished quiz (so Back can undo the last answer).
    Result(PlacementResult, Vec<String>, Option<Box<Session>>),
}

pub struct PlacementUi {
    stage: Stage,
    experience: Option<Experience>,
    claims: Vec<String>,
    seed: u64,
}

impl PlacementUi {
    /// The question being asked right now (used by tests).
    #[cfg(test)]
    pub fn current_item(&self) -> Option<&content::PlacementItem> {
        match &self.stage {
            Stage::Quiz(s) => s.current(),
            _ => None,
        }
    }

    pub fn new() -> PlacementUi {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(5);
        PlacementUi {
            stage: Stage::Experience,
            experience: None,
            claims: Vec::new(),
            seed,
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(mut p) = app.placement.take() else {
        app.placement = Some(PlacementUi::new());
        return;
    };
    let curriculum = app.curriculum.clone();
    ui.label(RichText::new("Find your starting point").font(theme::heading(30.0)));
    ui.add_space(6.0);
    let mut next_stage = None;
    card(ui, |ui| {
        ui.set_min_width(ui.available_width().min(820.0));
        match &mut p.stage {
            Stage::Experience => {
                ui.label(
                    RichText::new("How long have you been playing guitar?")
                        .font(theme::heading(22.0)),
                );
                ui.add_space(6.0);
                if kit::secondary(ui, Some(Icon::ChevronLeft), "Back").clicked() {
                    app.screen = if app.profile.as_ref().is_some_and(|p| p.placement_done) {
                        Screen::Settings
                    } else {
                        Screen::Welcome
                    };
                }
                ui.add_space(4.0);
                for exp in Experience::ALL {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(exp.label()).size(19.0))
                                .min_size(egui::vec2(360.0, 44.0)),
                        )
                        .clicked()
                    {
                        p.experience = Some(exp);
                        next_stage = Some(if exp == Experience::Never {
                            let session = Session::new(
                                &curriculum.placement,
                                &Profile {
                                    experience: exp,
                                    claims: vec![],
                                },
                                p.seed,
                            );
                            let result = session.result().cloned().unwrap();
                            Stage::Result(result, Vec::new(), None)
                        } else {
                            Stage::Claims(exp)
                        });
                    }
                }
            }
            Stage::Claims(exp) => {
                let exp = *exp;
                ui.label(
                    RichText::new("Which of these can you do without stopping to think?")
                        .font(theme::heading(22.0)),
                );
                ui.label(
                    RichText::new("Tick all that are true. It's fine to tick none!")
                        .color(theme::SOFT_INK),
                );
                ui.add_space(6.0);
                for claim in &curriculum.placement.claims {
                    let mut on = p.claims.contains(&claim.id);
                    if ui
                        .checkbox(&mut on, RichText::new(&claim.text).size(18.0))
                        .changed()
                    {
                        if on {
                            p.claims.push(claim.id.clone());
                        } else {
                            p.claims.retain(|c| c != &claim.id);
                        }
                    }
                }
                ui.add_space(8.0);
                if kit::primary(ui, None, "Continue ▶").clicked() {
                    let session = Session::new(
                        &curriculum.placement,
                        &Profile {
                            experience: exp,
                            claims: p.claims.clone(),
                        },
                        p.seed,
                    );
                    next_stage = Some(match session.result() {
                        Some(r) => {
                            Stage::Result(r.clone(), placed_out_lessons(&curriculum, r), None)
                        }
                        None => Stage::Quiz(Box::new(session)),
                    });
                }
                if kit::secondary(ui, Some(Icon::ChevronLeft), "Back").clicked() {
                    next_stage = Some(Stage::Experience);
                }
            }
            Stage::Quiz(session) => {
                ui.add(
                    egui::ProgressBar::new(session.progress())
                        .text(format!("Question {}", session.questions_asked() + 1)),
                );
                ui.add_space(6.0);
                if let Some(item) = session.current().cloned() {
                    let order = shuffled(
                        item.question.choices.len(),
                        p.seed ^ session.questions_asked() as u64,
                    );
                    let picked = question_card(app, ui, &item.question, &order, None, &item.id);
                    ui.add_space(6.0);
                    let (back, unsure) = ui
                        .horizontal(|ui| {
                            let back =
                                kit::secondary(ui, Some(Icon::ChevronLeft), "Back").clicked();
                            let unsure = ui
                                .button(RichText::new("I'm not sure").size(16.0))
                                .clicked();
                            (back, unsure)
                        })
                        .inner;
                    if let Some(i) = picked {
                        session.answer(i == item.question.answer);
                    } else if unsure {
                        session.answer(false);
                    } else if back && !session.back() {
                        next_stage = Some(Stage::Claims(
                            p.experience.unwrap_or(Experience::UnderSixMonths),
                        ));
                    }
                }
                if next_stage.is_none()
                    && let Some(r) = session.result()
                {
                    let r = r.clone();
                    let placed = placed_out_lessons(&curriculum, &r);
                    let done = std::mem::replace(
                        session,
                        Box::new(Session::new(
                            &curriculum.placement,
                            &Profile {
                                experience: Experience::Never,
                                claims: vec![],
                            },
                            0,
                        )),
                    );
                    next_stage = Some(Stage::Result(r, placed, Some(done)));
                }
            }
            Stage::Result(result, placed, finished_quiz) => {
                let level = curriculum.levels.iter().find(|l| l.number == result.level);
                match level {
                    Some(l) => {
                        ui.label(
                            RichText::new(format!(
                                "You're starting at Level {} · {}",
                                l.number, l.title
                            ))
                            .font(theme::heading(28.0))
                            .color(theme::GREEN),
                        );
                        ui.label(RichText::new(&l.goal).size(18.0));
                    }
                    None => {
                        ui.label(
                            RichText::new("Wow — you already know the whole core path!")
                                .font(theme::heading(28.0))
                                .color(theme::GREEN),
                        );
                    }
                }
                ui.add_space(6.0);
                if placed.is_empty() {
                    ui.label(RichText::new("We'll start from the first lesson.").size(17.0));
                } else {
                    ui.label(RichText::new(format!("We marked {} lessons as already known. They show a blue ✔ — open any of them to review.", placed.len())).size(17.0));
                }
                let written_up_to = curriculum
                    .levels
                    .iter()
                    .filter(|l| l.lessons.iter().any(|x| !x.coming_soon))
                    .map(|l| l.number)
                    .max()
                    .unwrap_or(0);
                if result.level > written_up_to {
                    ui.label(RichText::new(format!("Lessons for Level {} are still being written. For now you can review earlier levels and use Chords, Neck Explorer, and the Tuner to practice.", result.level.min(7))).color(theme::ORANGE).size(16.0));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if kit::secondary(ui, Some(Icon::ChevronLeft), "Back").clicked() {
                        next_stage = Some(match finished_quiz.take() {
                            Some(mut quiz) => {
                                quiz.back();
                                Stage::Quiz(quiz)
                            }
                            None => match p.experience {
                                Some(Experience::Never) | None => Stage::Experience,
                                Some(exp) => Stage::Claims(exp),
                            },
                        });
                    }
                    if kit::primary(ui, None, "Go to My Path ▶").clicked() {
                        let list = placed.clone();
                        app.complete_placement(&list);
                        app.screen = Screen::Path;
                    }
                });
            }
        }
    });
    if let Some(s) = next_stage {
        p.stage = s;
    }
    if app.screen == Screen::Placement {
        app.placement = Some(p);
    }
}
