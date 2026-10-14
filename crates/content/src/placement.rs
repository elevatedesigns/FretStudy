//! The placement assessment (docs/03-Placement-Assessment.md):
//! Stage A (quick profile) → Stage B (adaptive level search) → Stage C (pinpoint the unit).

use crate::{Curriculum, PlacementBank, PlacementItem, Thread};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Experience {
    Never,
    UnderSixMonths,
    SixMonthsToTwoYears,
    TwoPlusYears,
}

impl Experience {
    pub const ALL: [Experience; 4] = [
        Experience::Never,
        Experience::UnderSixMonths,
        Experience::SixMonthsToTwoYears,
        Experience::TwoPlusYears,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Experience::Never => "Never — I'm brand new",
            Experience::UnderSixMonths => "Less than 6 months",
            Experience::SixMonthsToTwoYears => "6 months to 2 years",
            Experience::TwoPlusYears => "More than 2 years",
        }
    }

    /// Highest level the starting guess may reach.
    fn cap(self) -> u8 {
        match self {
            Experience::Never => 0,
            Experience::UnderSixMonths => 3,
            Experience::SixMonthsToTwoYears => 5,
            Experience::TwoPlusYears => 7,
        }
    }
}

/// Stage A answers.
#[derive(Clone, Debug)]
pub struct Profile {
    pub experience: Experience,
    /// Ids of the "I can do this" claims the learner ticked.
    pub claims: Vec<String>,
}

/// Highest level where at least half of that level's claims were ticked (and every lower
/// level with claims has at least one), capped by experience.
pub fn starting_guess(bank: &PlacementBank, profile: &Profile) -> u8 {
    let mut guess = 0u8;
    for level in 1..=7u8 {
        let at_level: Vec<_> = bank.claims.iter().filter(|c| c.level == level).collect();
        if at_level.is_empty() {
            continue;
        }
        let ticked = at_level
            .iter()
            .filter(|c| profile.claims.contains(&c.id))
            .count();
        if ticked * 2 >= at_level.len() {
            guess = level;
        } else if ticked == 0 {
            break;
        }
    }
    guess.min(profile.experience.cap())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacementResult {
    /// Start in this level (8 = all core levels known).
    pub level: u8,
    /// Threads of `level` the learner already showed they know.
    pub known_threads: Vec<Thread>,
    pub questions_asked: usize,
}

/// Stops Stage B after this many questions even if the boundary isn't pinned down.
const STAGE_B_LIMIT: usize = 8;
const STAGE_C_LIMIT: usize = 2;
const TOP: u8 = 8;
const PINPOINT_ORDER: [Thread; 5] = [
    Thread::Chords,
    Thread::Rhythm,
    Thread::Notes,
    Thread::TheoryEar,
    Thread::Technique,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Search,
    Pinpoint,
    Done,
}

/// A running placement quiz. Ask `current()`, report `answer()`, until `result()` is Some.
pub struct Session {
    bank: PlacementBank,
    profile: Profile,
    seed: u64,
    items: Vec<PlacementItem>,
    stage: Stage,
    level: u8,
    highest_passed: i16,
    lowest_failed: u8,
    round_asked: Vec<usize>,
    round_correct: usize,
    asked: Vec<(usize, bool)>,
    stage_b_count: usize,
    pinpoint_count: usize,
    known_threads: Vec<Thread>,
    current: Option<usize>,
    rng: u64,
    result: Option<PlacementResult>,
}

impl Session {
    pub fn new(bank: &PlacementBank, profile: &Profile, seed: u64) -> Session {
        let guess = starting_guess(bank, profile);
        let mut s = Session {
            bank: bank.clone(),
            profile: profile.clone(),
            seed,
            items: bank.items.clone(),
            stage: Stage::Search,
            level: guess,
            highest_passed: -1,
            lowest_failed: TOP,
            round_asked: Vec::new(),
            round_correct: 0,
            asked: Vec::new(),
            stage_b_count: 0,
            pinpoint_count: 0,
            known_threads: Vec::new(),
            current: None,
            rng: seed | 1,
            result: None,
        };
        if profile.experience == Experience::Never && profile.claims.is_empty() {
            s.finish(0);
        } else {
            s.pick_search_item();
        }
        s
    }

    /// Undoes the last answer (the quiz is adaptive, so it replays the earlier answers).
    /// Returns false when there's nothing to undo.
    pub fn back(&mut self) -> bool {
        let answers: Vec<bool> = self.asked.iter().map(|(_, ok)| *ok).collect();
        if answers.is_empty() {
            return false;
        }
        let mut replay = Session::new(&self.bank, &self.profile, self.seed);
        for ok in &answers[..answers.len() - 1] {
            replay.answer(*ok);
        }
        *self = replay;
        true
    }

    pub fn current(&self) -> Option<&PlacementItem> {
        self.current.map(|i| &self.items[i])
    }

    pub fn result(&self) -> Option<&PlacementResult> {
        self.result.as_ref()
    }

    pub fn questions_asked(&self) -> usize {
        self.asked.len()
    }

    /// Rough progress (0..1) for a progress bar.
    pub fn progress(&self) -> f32 {
        if self.result.is_some() {
            1.0
        } else {
            (self.asked.len() as f32 / 10.0).min(0.95)
        }
    }

    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn already_asked(&self, i: usize) -> bool {
        self.asked.iter().any(|(j, _)| *j == i)
    }

    /// Picks an unasked item at the current level, preferring a thread not yet used this round.
    fn pick_search_item(&mut self) {
        let used_threads: Vec<Thread> = self
            .round_asked
            .iter()
            .map(|&i| self.items[i].thread)
            .collect();
        let mut candidates: Vec<usize> = (0..self.items.len())
            .filter(|&i| self.items[i].level == self.level && !self.already_asked(i))
            .collect();
        if candidates.is_empty() {
            // Out of items for this level: decide with what we have.
            let passed = self.round_correct * 2 >= self.round_asked.len().max(1)
                && !self.round_asked.is_empty();
            self.end_round(passed);
            return;
        }
        let fresh: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&i| !used_threads.contains(&self.items[i].thread))
            .collect();
        if !fresh.is_empty() {
            candidates = fresh;
        }
        let pick = candidates[(self.rand() % candidates.len() as u64) as usize];
        self.current = Some(pick);
    }

    pub fn answer(&mut self, correct: bool) {
        let Some(i) = self.current.take() else { return };
        self.asked.push((i, correct));
        match self.stage {
            Stage::Search => {
                self.stage_b_count += 1;
                self.round_asked.push(i);
                if correct {
                    self.round_correct += 1;
                }
                let n = self.round_asked.len();
                let c = self.round_correct;
                let decided = match (n, c) {
                    (2, 2) => Some(true),
                    (2, 0) => Some(false),
                    (3, c) => Some(c >= 2),
                    _ => None,
                };
                match decided {
                    Some(passed) => self.end_round(passed),
                    None if self.stage_b_count >= STAGE_B_LIMIT => self.end_round(c * 2 >= n),
                    None => self.pick_search_item(),
                }
            }
            Stage::Pinpoint => {
                self.pinpoint_count += 1;
                if correct {
                    let t = self.items[i].thread;
                    if !self.known_threads.contains(&t) {
                        self.known_threads.push(t);
                    }
                    self.pick_pinpoint_item();
                } else {
                    let level = self.level;
                    self.finish(level);
                }
            }
            Stage::Done => {}
        }
    }

    fn end_round(&mut self, passed: bool) {
        let level = self.level;
        if passed {
            self.highest_passed = self.highest_passed.max(level as i16);
        } else {
            self.lowest_failed = self.lowest_failed.min(level);
            // Threads answered correctly at the failed level still count in Stage C.
            for &(i, ok) in &self.asked {
                if ok
                    && self.items[i].level == level
                    && !self.known_threads.contains(&self.items[i].thread)
                {
                    self.known_threads.push(self.items[i].thread);
                }
            }
        }
        self.round_asked.clear();
        self.round_correct = 0;
        let lo = (self.highest_passed + 1) as u8;
        let hi = self.lowest_failed;
        if hi <= lo || self.stage_b_count >= STAGE_B_LIMIT {
            let target = lo.min(TOP);
            self.start_pinpoint(target);
            return;
        }
        // Halve the unknown range each round (binary search), so any of the 9 levels is
        // found in about 4 rounds.
        let next = if passed {
            (level + ((hi - level) / 2).max(1)).min(hi - 1)
        } else {
            lo + (level - lo) / 2
        };
        self.level = next;
        if self.level < lo || self.level >= hi {
            self.start_pinpoint(lo.min(TOP));
        } else {
            self.pick_search_item();
        }
    }

    fn start_pinpoint(&mut self, target: u8) {
        self.level = target;
        if target >= TOP {
            self.finish(TOP);
            return;
        }
        // Threads learned about at levels other than the target don't carry over.
        let target_threads: Vec<Thread> = self
            .asked
            .iter()
            .filter(|(i, ok)| *ok && self.items[*i].level == target)
            .map(|(i, _)| self.items[*i].thread)
            .collect();
        self.known_threads.retain(|t| target_threads.contains(t));
        self.stage = Stage::Pinpoint;
        self.pick_pinpoint_item();
    }

    fn pick_pinpoint_item(&mut self) {
        if self.pinpoint_count >= STAGE_C_LIMIT {
            let level = self.level;
            self.finish(level);
            return;
        }
        for thread in PINPOINT_ORDER {
            if self.known_threads.contains(&thread) {
                continue;
            }
            if let Some(i) = (0..self.items.len()).find(|&i| {
                self.items[i].level == self.level
                    && self.items[i].thread == thread
                    && !self.already_asked(i)
            }) {
                self.current = Some(i);
                return;
            }
        }
        let level = self.level;
        self.finish(level);
    }

    fn finish(&mut self, level: u8) {
        self.stage = Stage::Done;
        self.current = None;
        let known_threads = if level >= TOP {
            Vec::new()
        } else {
            self.known_threads.clone()
        };
        self.result = Some(PlacementResult {
            level,
            known_threads,
            questions_asked: self.asked.len(),
        });
    }
}

/// Lessons to mark "placed out": every lesson in lower levels, plus lessons in the start
/// level whose thread the learner already showed (never the level's song).
pub fn placed_out_lessons(curriculum: &Curriculum, result: &PlacementResult) -> Vec<String> {
    curriculum
        .levels
        .iter()
        .flat_map(|level| {
            level
                .lessons
                .iter()
                .map(move |lesson| (level.number, lesson))
        })
        .filter(|(n, lesson)| {
            *n < result.level
                || (*n == result.level
                    && lesson.thread != Thread::Song
                    && result.known_threads.contains(&lesson.thread))
        })
        .map(|(_, lesson)| lesson.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Curriculum;

    fn bank() -> PlacementBank {
        Curriculum::load().unwrap().placement
    }

    /// A learner who knows everything below `true_level` and nothing at or above it.
    fn run(bank: &PlacementBank, profile: &Profile, true_level: u8, seed: u64) -> PlacementResult {
        let mut s = Session::new(bank, profile, seed);
        let mut guard = 0;
        while let Some(item) = s.current() {
            let correct = item.level < true_level;
            s.answer(correct);
            guard += 1;
            assert!(guard < 30, "placement never finished");
        }
        s.result().unwrap().clone()
    }

    #[test]
    fn brand_new_learners_skip_the_quiz() {
        let b = bank();
        let p = Profile {
            experience: Experience::Never,
            claims: vec![],
        };
        let s = Session::new(&b, &p, 1);
        assert!(s.current().is_none());
        assert_eq!(s.result().unwrap().level, 0);
        assert_eq!(s.result().unwrap().questions_asked, 0);
    }

    #[test]
    fn starting_guesses() {
        let b = bank();
        let p = |exp, claims: &[&str]| Profile {
            experience: exp,
            claims: claims.iter().map(|s| s.to_string()).collect(),
        };
        assert_eq!(starting_guess(&b, &p(Experience::Never, &["tune"])), 0); // capped by experience
        assert_eq!(
            starting_guess(&b, &p(Experience::UnderSixMonths, &["tune", "open5"])),
            2
        );
        assert_eq!(
            starting_guess(
                &b,
                &p(
                    Experience::UnderSixMonths,
                    &["tune", "open5", "tab", "strum", "barre"]
                )
            ),
            3
        );
        assert_eq!(
            starting_guess(
                &b,
                &p(
                    Experience::TwoPlusYears,
                    &["tune", "open5", "tab", "strum", "barre", "penta"]
                )
            ),
            5
        );
    }

    #[test]
    fn finds_the_right_level_from_any_starting_guess() {
        let b = bank();
        let all_claims: Vec<String> = b.claims.iter().map(|c| c.id.clone()).collect();
        for true_level in 0..=8u8 {
            for claims_up_to in 0..=7u8 {
                let claims: Vec<String> = b
                    .claims
                    .iter()
                    .filter(|c| c.level <= claims_up_to)
                    .map(|c| c.id.clone())
                    .collect();
                for exp in [
                    Experience::UnderSixMonths,
                    Experience::SixMonthsToTwoYears,
                    Experience::TwoPlusYears,
                ] {
                    for seed in [1, 7, 42] {
                        let p = Profile {
                            experience: exp,
                            claims: claims.clone(),
                        };
                        let r = run(&b, &p, true_level, seed);
                        assert_eq!(
                            r.level, true_level,
                            "true {true_level}, guess claims ≤{claims_up_to}, {exp:?}"
                        );
                        assert!(
                            r.questions_asked <= 10,
                            "{} questions (true {true_level})",
                            r.questions_asked
                        );
                        // Learners at level N know nothing at level N, so no threads are placed out.
                        assert!(r.known_threads.is_empty());
                    }
                }
            }
        }
        let _ = all_claims;
    }

    #[test]
    fn pinpoint_places_out_known_threads() {
        let b = bank();
        let c = Curriculum::load().unwrap();
        // Knows Levels 0–3 and Level 4 chords, but not the rest of Level 4.
        let p = Profile {
            experience: Experience::TwoPlusYears,
            claims: vec![
                "tune".into(),
                "open5".into(),
                "tab".into(),
                "strum".into(),
                "barre".into(),
            ],
        };
        let mut s = Session::new(&b, &p, 3);
        while let Some(item) = s.current() {
            let correct = item.level < 4 || (item.level == 4 && item.thread == Thread::Chords);
            s.answer(correct);
        }
        let r = s.result().unwrap().clone();
        assert_eq!(r.level, 4);
        assert_eq!(r.known_threads, vec![Thread::Chords]);
        let out = placed_out_lessons(&c, &r);
        assert!(out.contains(&"L3.13".to_string()));
        assert!(
            out.contains(&"L4.3".to_string()),
            "power chords (L4 chords) placed out"
        );
        assert!(
            !out.contains(&"L4.1".to_string()),
            "L4 notes not placed out"
        );
        assert!(!out.contains(&"L4.13".to_string()), "never the level song");
    }

    #[test]
    fn going_back_undoes_the_last_answer() {
        let b = bank();
        let p = Profile {
            experience: Experience::TwoPlusYears,
            claims: vec!["tune".into(), "open5".into()],
        };
        let mut s = Session::new(&b, &p, 11);
        let first = s.current().unwrap().id.clone();
        s.answer(true);
        let second = s.current().unwrap().id.clone();
        s.answer(false);
        assert!(s.back());
        assert_eq!(
            s.current().unwrap().id,
            second,
            "back shows the previous question again"
        );
        assert_eq!(s.questions_asked(), 1);
        assert!(s.back());
        assert_eq!(s.current().unwrap().id, first);
        assert!(!s.back(), "nothing left to undo");
    }

    #[test]
    fn total_questions_stay_short() {
        // Including the 2 Stage A questions, the whole quiz stays at 12 or fewer.
        let b = bank();
        let p = Profile {
            experience: Experience::TwoPlusYears,
            claims: b.claims.iter().map(|c| c.id.clone()).collect(),
        };
        for true_level in 0..=8 {
            let r = run(&b, &p, true_level, 9);
            assert!(
                r.questions_asked + 2 <= 12,
                "{} questions",
                r.questions_asked
            );
        }
    }
}
