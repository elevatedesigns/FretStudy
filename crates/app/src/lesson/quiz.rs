//! Multiple-choice questions, shared by Check It steps and the placement quiz.

use super::visuals;
use crate::app::App;
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit;
use content::{Audio, Question};
use egui::{RichText, Ui};
use theory::library;

/// A small deterministic shuffle so answers aren't always in the same spot.
pub fn shuffled(n: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    for i in (1..n).rev() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        order.swap(i, (x % (i as u64 + 1)) as usize);
    }
    order
}

pub fn play_audio(app: &mut App, audio: &Audio) {
    match audio {
        Audio::Chords(ids) => {
            let shapes: Vec<_> = ids
                .iter()
                .filter_map(|id| library::find(id))
                .map(|e| e.shape())
                .collect();
            let tuning = app.tuning.clone();
            app.sound.chords(&tuning, &shapes, 1.4);
        }
        Audio::Notes(notes) => {
            let pos = visuals::tab_timed(notes);
            let tuning = app.tuning.clone();
            app.sound.play_tab(&tuning, &pos, 50.0);
        }
    }
}

/// Shows one question. Returns the original index of the choice clicked this frame.
/// With `reveal = Some(picked)`, shows right/wrong and the explanation instead of buttons.
pub fn question_card(
    app: &mut App,
    ui: &mut Ui,
    q: &Question,
    order: &[usize],
    reveal: Option<usize>,
    salt: &str,
) -> Option<usize> {
    let mut clicked = None;
    ui.label(RichText::new(&q.prompt).font(theme::heading(22.0)));
    ui.add_space(6.0);
    if let Some(v) = &q.visual {
        // In "name this chord" questions, never show the chord's name above the diagram.
        let hidden;
        let v = match v {
            content::Visual::Chord(id) if q.prompt.starts_with("Name this") => {
                match library::find(id) {
                    Some(e) => {
                        hidden = content::Visual::Shape {
                            frets: e.frets.to_string(),
                            fingers: e.fingers.to_string(),
                            name: String::new(),
                        };
                        &hidden
                    }
                    None => v,
                }
            }
            _ => v,
        };
        visuals::show(app, ui, v, salt);
        ui.add_space(6.0);
    }
    if let Some(a) = &q.audio
        && kit::secondary(ui, None, "🔊 Listen").clicked()
    {
        play_audio(app, a);
    }
    ui.add_space(8.0);
    for &i in order {
        let text = theme::glyphs(&q.choices[i]);
        match reveal {
            None => {
                let b = egui::Button::new(RichText::new(text).size(18.0))
                    .min_size(egui::vec2(320.0, 40.0));
                if ui.add(b).clicked() {
                    clicked = Some(i);
                }
            }
            Some(picked) => {
                let (mark, color) = if i == q.answer {
                    ("✔ ", theme::GREEN)
                } else if i == picked {
                    ("✖ ", theme::VERMILLION)
                } else {
                    ("   ", theme::SOFT_INK)
                };
                ui.label(
                    RichText::new(format!("{mark}{text}"))
                        .size(18.0)
                        .color(color),
                );
            }
        }
    }
    if let Some(picked) = reveal {
        ui.add_space(8.0);
        let right = picked == q.answer;
        let (fill, ink) = if right {
            (theme::GREEN_SOFT, theme::GREEN)
        } else {
            (
                theme::YELLOW_SOFT,
                egui::Color32::from_rgb(0xA3, 0x4A, 0x00),
            )
        };
        egui::Frame::new()
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_max_width(640.0);
                if right {
                    ui.label(
                        RichText::new("✔ That's right!")
                            .color(ink)
                            .size(18.0)
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new("Not quite — and that's how we learn!")
                            .color(ink)
                            .size(18.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(theme::glyphs(&format!(
                            "The answer is: {}",
                            q.choices[q.answer]
                        )))
                        .size(17.0)
                        .strong(),
                    );
                }
                if !q.explain.is_empty() {
                    ui.label(RichText::new(theme::glyphs(&q.explain)).size(17.0));
                }
            });
    }
    clicked
}

/// A Check It quiz: one question at a time with feedback, then a score.
pub struct Quiz {
    pub questions: Vec<Question>,
    pass_percent: u8,
    index: usize,
    /// The answer picked for each question so far.
    picks: Vec<Option<usize>>,
    attempt: u64,
    pub finished: Option<f32>,
}

pub enum QuizOutcome {
    Passed(f32),
    Failed(f32),
}

impl Quiz {
    pub fn new(questions: Vec<Question>, pass_percent: u8) -> Quiz {
        let n = questions.len();
        Quiz {
            questions,
            pass_percent,
            index: 0,
            picks: vec![None; n],
            attempt: 1,
            finished: None,
        }
    }

    fn correct(&self) -> usize {
        self.questions
            .iter()
            .zip(&self.picks)
            .filter(|(q, p)| **p == Some(q.answer))
            .count()
    }

    fn restart(&mut self) {
        self.index = 0;
        self.picks = vec![None; self.questions.len()];
        self.attempt += 1;
        self.finished = None;
    }

    /// Returns an outcome on the frame the quiz is finished.
    pub fn show(&mut self, app: &mut App, ui: &mut Ui) -> Option<QuizOutcome> {
        let total = self.questions.len();
        if let Some(score) = self.finished {
            let passed = score * 100.0 + 0.01 >= self.pass_percent as f32;
            ui.label(
                RichText::new(format!(
                    "You got {} of {} ({:.0}%).",
                    self.correct(),
                    total,
                    score * 100.0
                ))
                .font(theme::heading(24.0)),
            );
            if !passed {
                ui.label(RichText::new(format!("You need {}% to finish this lesson. Look back at any step, then try again. You've got this!", self.pass_percent)).size(17.0));
                if kit::secondary(ui, None, "Try again").clicked() {
                    self.restart();
                }
            }
            return None;
        }
        ui.label(
            RichText::new(format!("Question {} of {}", self.index + 1, total))
                .color(theme::SOFT_INK),
        );
        let q = self.questions[self.index].clone();
        let order = shuffled(q.choices.len(), self.attempt * 1000 + self.index as u64);
        let salt = format!("q{}-{}", self.attempt, self.index);
        let picked = self.picks[self.index];
        if let Some(i) = question_card(app, ui, &q, &order, picked, &salt)
            && picked.is_none()
        {
            self.picks[self.index] = Some(i);
        }
        ui.add_space(8.0);
        let mut outcome = None;
        ui.horizontal(|ui| {
            // Back reviews the previous question (answers show right away, so they stay locked in).
            if self.index > 0 && kit::secondary(ui, Some(Icon::ChevronLeft), "Back").clicked() {
                self.index -= 1;
            }
            if self.picks[self.index].is_some() {
                let last = self.index + 1 == total;
                if ui
                    .button(
                        RichText::new(if last {
                            "See my score"
                        } else {
                            "Next question ▶"
                        })
                        .size(18.0),
                    )
                    .clicked()
                {
                    if last {
                        let score = self.correct() as f32 / total as f32;
                        self.finished = Some(score);
                        let passed = score * 100.0 + 0.01 >= self.pass_percent as f32;
                        outcome = Some(if passed {
                            QuizOutcome::Passed(score)
                        } else {
                            QuizOutcome::Failed(score)
                        });
                    } else {
                        self.index += 1;
                    }
                }
            }
        });
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::shuffled;

    #[test]
    fn shuffle_is_a_permutation_and_varies() {
        for n in 2..6 {
            for seed in 0..20 {
                let mut o = shuffled(n, seed);
                o.sort();
                assert_eq!(o, (0..n).collect::<Vec<_>>());
            }
        }
        let firsts: std::collections::HashSet<usize> = (0..30).map(|s| shuffled(3, s)[0]).collect();
        assert!(
            firsts.len() == 3,
            "the right answer isn't stuck in one place"
        );
    }
}
