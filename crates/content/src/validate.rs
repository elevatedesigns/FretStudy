//! Content validator (runs as tests): catches mistakes in lessons before learners see them.

use crate::readability::grade_level;
use crate::*;
use std::collections::HashSet;
use theory::library;
use theory::{Chord, ChordShape, Tuning};

fn curriculum() -> Curriculum {
    Curriculum::load().expect("content parses")
}

fn visuals_in(step: &Step) -> Vec<&Visual> {
    match step {
        Step::Learn { visual, .. } => visual.iter().collect(),
        Step::Check { questions, .. } => {
            questions.iter().filter_map(|q| q.visual.as_ref()).collect()
        }
        Step::Activity { .. } => Vec::new(),
    }
}

fn questions_in(step: &Step) -> Vec<&Question> {
    match step {
        Step::Check { questions, .. } => questions.iter().collect(),
        _ => Vec::new(),
    }
}

/// Library chord ids an activity asks the learner to play.
fn chords_played(activity: &Activity) -> Vec<&str> {
    match activity {
        Activity::StringCheck { chord } => vec![chord],
        Activity::StrumAlong { chords, .. } => chords.iter().map(String::as_str).collect(),
        Activity::ChangeTrainer { a, b, .. } => vec![a, b],
        Activity::ChordCheck { chords } => chords.iter().map(String::as_str).collect(),
        Activity::Jam { chords, .. } => chords.iter().map(String::as_str).collect(),
        Activity::Record { chords, .. } => chords.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

fn check_visual(v: &Visual, where_: &str) {
    match v {
        Visual::Chord(id) => assert!(
            library::find(id).is_some(),
            "{where_}: unknown chord id {id}"
        ),
        Visual::ChordPair(a, b) => {
            assert!(library::find(a).is_some(), "{where_}: unknown chord id {a}");
            assert!(library::find(b).is_some(), "{where_}: unknown chord id {b}");
        }
        Visual::Shape { frets, fingers, .. } => {
            ChordShape::parse(frets, Some(fingers)).unwrap_or_else(|e| panic!("{where_}: {e}"));
        }
        Visual::Neck {
            notes,
            first_fret,
            last_fret,
            ..
        } => {
            assert!(first_fret < last_fret, "{where_}: bad fret range");
            for n in notes {
                assert!((1..=6).contains(&n.string), "{where_}: string {}", n.string);
                assert!(
                    n.fret <= 22 && n.fret >= *first_fret && n.fret <= *last_fret,
                    "{where_}: fret {} outside the drawn range",
                    n.fret
                );
            }
        }
        Visual::Tab(notes) => {
            for n in notes {
                assert!(
                    (1..=6).contains(&n.string) && n.fret <= 22,
                    "{where_}: bad tab note"
                );
            }
        }
        Visual::BeatPulse { bpm } => assert!((30.0..=240.0).contains(bpm)),
        Visual::ScaleCompare {
            root,
            a,
            b,
            first_fret,
            last_fret,
        } => {
            root.parse::<theory::NoteName>()
                .unwrap_or_else(|_| panic!("{where_}: bad root {root}"));
            assert!(
                scale_kind(a).is_some() && scale_kind(b).is_some(),
                "{where_}: unknown scale"
            );
            assert!(
                first_fret < last_fret && *last_fret <= 22,
                "{where_}: fret range"
            );
        }
        Visual::Staff { notes, key, bar } => {
            assert!((2..=6).contains(bar), "{where_}: beats in a bar");
            assert!(!notes.is_empty(), "{where_}: staff needs notes");
            if let Some(k) = key {
                k.parse::<theory::NoteName>()
                    .unwrap_or_else(|_| panic!("{where_}: bad key {k}"));
            }
        }
        Visual::Decoder { examples } => {
            assert!(!examples.is_empty(), "{where_}: decoder needs examples");
            for e in examples {
                e.replace('♯', "#")
                    .replace('♭', "b")
                    .parse::<Chord>()
                    .unwrap_or_else(|_| panic!("{where_}: can't read chord symbol {e}"));
            }
        }
        Visual::ScaleSteps { root } | Visual::NumberWheel { key: root, .. } => {
            root.parse::<theory::NoteName>()
                .unwrap_or_else(|_| panic!("{where_}: bad key {root}"));
        }
        Visual::ScaleBox {
            root,
            scale,
            shape,
            first_fret,
            last_fret,
        } => {
            root.parse::<theory::NoteName>()
                .unwrap_or_else(|_| panic!("{where_}: bad root {root}"));
            let kind =
                scale_kind(scale).unwrap_or_else(|| panic!("{where_}: unknown scale {scale}"));
            if let Some(n) = shape {
                assert!((1..=5).contains(n), "{where_}: shape {n}");
                assert!(
                    matches!(
                        kind,
                        theory::ScaleKind::MinorPentatonic
                            | theory::ScaleKind::MajorPentatonic
                            | theory::ScaleKind::Blues
                    ),
                    "{where_}: shapes are for pentatonic scales"
                );
            }
            assert!(
                first_fret < last_fret && *last_fret <= 22,
                "{where_}: fret range"
            );
        }
        Visual::Caged { root } | Visual::TwelveBar { key: root } => {
            root.parse::<theory::NoteName>()
                .unwrap_or_else(|_| panic!("{where_}: bad root {root}"));
        }
        Visual::Capo { fret, chord } => {
            assert!(
                library::find(chord).is_some(),
                "{where_}: unknown chord id {chord}"
            );
            assert!((1..=9).contains(fret), "{where_}: capo fret {fret}");
        }
        _ => {}
    }
}

fn check_question(q: &Question, where_: &str, lesson: bool) {
    assert!(q.choices.len() >= 2, "{where_}: needs at least 2 choices");
    // Every lesson question explains its answer, so a wrong pick still teaches something.
    assert!(
        !lesson || q.explain.split_whitespace().count() >= 6,
        "{where_}: \"{}\" needs an explanation",
        q.prompt
    );
    assert!(
        q.answer < q.choices.len(),
        "{where_}: answer index out of range"
    );
    let unique: HashSet<&String> = q.choices.iter().collect();
    assert_eq!(unique.len(), q.choices.len(), "{where_}: duplicate choices");
    if let Some(v) = &q.visual {
        check_visual(v, where_);
    }
    if let Some(Audio::Chords(ids)) = &q.audio {
        for id in ids {
            assert!(
                library::find(id).is_some(),
                "{where_}: unknown chord id {id} in audio"
            );
        }
    }
    // "Name this chord" questions: the right answer must match what the shape really spells.
    if q.prompt.starts_with("Name this") {
        let shape = match &q.visual {
            Some(Visual::Chord(id)) => library::find(id).map(|e| e.shape()),
            Some(Visual::Shape { frets, fingers, .. }) => {
                ChordShape::parse(frets, Some(fingers)).ok()
            }
            _ => None,
        };
        if let Some(shape) = shape {
            let t = Tuning::standard();
            let answer = normalize_chord_name(&q.choices[q.answer]);
            let chord: Chord = answer
                .parse()
                .unwrap_or_else(|_| panic!("{where_}: can't read answer {answer:?} as a chord"));
            assert!(
                shape.check(&chord, &t).ok(),
                "{where_}: shape {} does not spell {}",
                shape,
                q.choices[q.answer]
            );
        }
    }
}

/// "E minor" → "Em", "Easy C" → "C", "B♭ major" → "Bb", "E minor 7" → "Em7".
fn normalize_chord_name(s: &str) -> String {
    let s = s.replace("Easy ", "").replace('♯', "#").replace('♭', "b");
    let s = s
        .replace(" minor 7", "m7")
        .replace(" major 7", "maj7")
        .replace(" minor", "m")
        .replace(" major", "");
    s.split_whitespace().next().unwrap_or("").to_string()
}

#[test]
fn content_parses_and_ids_are_unique() {
    let c = curriculum();
    // Core levels 0–7, then the style tracks (numbered from 10, each marked as a track).
    let core: Vec<u8> = c
        .levels
        .iter()
        .filter(|l| l.track.is_none())
        .map(|l| l.number)
        .collect();
    assert_eq!(core, (0..=7).collect::<Vec<_>>());
    for t in c.levels.iter().filter(|l| l.track.is_some()) {
        assert!(
            t.number >= 10,
            "track {} must be numbered 10 or more",
            t.title
        );
    }
    let numbers: HashSet<u8> = c.levels.iter().map(|l| l.number).collect();
    assert_eq!(
        numbers.len(),
        c.levels.len(),
        "level numbers must be unique"
    );
    let mut seen = HashSet::new();
    for l in c.lessons() {
        assert!(seen.insert(l.id.clone()), "duplicate lesson id {}", l.id);
    }
}

#[test]
fn prerequisites_point_backwards() {
    let c = curriculum();
    for l in c.lessons() {
        let me = c.order(&l.id).unwrap();
        for p in &l.prerequisites {
            let them = c
                .order(p)
                .unwrap_or_else(|| panic!("{}: unknown prerequisite {p}", l.id));
            assert!(
                them < me,
                "{}: prerequisite {p} comes later on the path",
                l.id
            );
        }
    }
}

#[test]
fn written_lessons_are_complete() {
    let c = curriculum();
    for l in c.lessons().filter(|l| !l.coming_soon) {
        assert!(!l.steps.is_empty(), "{} has no steps", l.id);
        assert!(!l.goal.is_empty(), "{} has no goal", l.id);
        assert!(
            l.tuning
                .as_deref()
                .is_none_or(|t| ["drop_d", "open_d", "open_g", "dadgad"].contains(&t)),
            "{}: unknown tuning",
            l.id
        );
        let checks = l
            .steps
            .iter()
            .filter(|s| matches!(s, Step::Check { .. }))
            .count();
        assert_eq!(checks, 1, "{} needs exactly one Check It step", l.id);
        assert!(
            matches!(l.steps.last(), Some(Step::Check { .. })),
            "{}: Check It comes last",
            l.id
        );
        for (i, s) in l.steps.iter().enumerate() {
            let where_ = format!("{} step {}", l.id, i + 1);
            for v in visuals_in(s) {
                check_visual(v, &where_);
            }
            for q in questions_in(s) {
                check_question(q, &where_, true);
            }
            if let Step::Activity { activity, .. } = s {
                match activity {
                    Activity::NoteHunt {
                        strings,
                        max_fret,
                        rounds,
                        ..
                    } => {
                        assert!(
                            !strings.is_empty() && strings.iter().all(|n| (1..=6).contains(n)),
                            "{where_}: note hunt strings"
                        );
                        assert!(*max_fret <= 15 && *rounds > 0, "{where_}: note hunt range");
                    }
                    Activity::BendCheck {
                        string,
                        fret,
                        steps,
                    } => {
                        assert!(
                            (1..=6).contains(string)
                                && *fret >= 1
                                && *fret <= 20
                                && (1..=2).contains(steps),
                            "{where_}: bend"
                        );
                    }
                    Activity::VibratoCheck { string, fret } => {
                        assert!((1..=6).contains(string) && *fret <= 20, "{where_}: vibrato");
                    }
                    Activity::Jam {
                        key,
                        scale,
                        shape,
                        chords,
                        bpm,
                        ..
                    } => {
                        key.parse::<theory::NoteName>()
                            .unwrap_or_else(|_| panic!("{where_}: bad key {key}"));
                        assert!(
                            scale_kind(scale).is_some(),
                            "{where_}: unknown scale {scale}"
                        );
                        assert!(
                            shape.is_none_or(|n| (1..=5).contains(&n)),
                            "{where_}: shape"
                        );
                        assert!(
                            !chords.is_empty() && (40.0..=200.0).contains(bpm),
                            "{where_}: jam backing"
                        );
                    }
                    Activity::Detective { key, rounds } => {
                        key.parse::<theory::NoteName>()
                            .unwrap_or_else(|_| panic!("{where_}: bad key {key}"));
                        assert!(*rounds > 0, "{where_}: rounds");
                    }
                    Activity::SongBuilder {
                        verse_bars,
                        chorus_bars,
                    } => {
                        assert!(*verse_bars > 0 && *chorus_bars > 0, "{where_}: song size");
                    }
                    Activity::Expression { notes } => {
                        assert!(!notes.is_empty(), "{where_}: expression needs notes");
                    }
                    Activity::Record { bpm, .. } => {
                        assert!(
                            (40.0..=200.0).contains(bpm),
                            "{where_}: record backing tempo"
                        );
                    }
                    Activity::PlayNotes { notes, .. } => {
                        for n in notes {
                            if let Some(Tech::Bend(b)) = n.tech {
                                assert!(
                                    (1..=2).contains(&b),
                                    "{where_}: bends are 1 or 2 half steps"
                                );
                            }
                        }
                        assert!(
                            !matches!(
                                notes.first().and_then(|n| n.tech),
                                Some(Tech::Hammer | Tech::Pull | Tech::Slide)
                            ),
                            "{where_}: a legato note needs a note before it"
                        );
                    }
                    _ => {}
                }
            }
            if let Step::Check {
                pass_percent,
                questions,
            } = s
            {
                assert!(
                    (50..=100).contains(pass_percent) && !questions.is_empty(),
                    "{where_}"
                );
            }
        }
    }
}

#[test]
fn activities_only_use_chords_already_taught() {
    let c = curriculum();
    for l in c.lessons() {
        let me = c.order(&l.id).unwrap();
        for s in &l.steps {
            if let Step::Activity { activity, .. } = s {
                for id in chords_played(activity) {
                    let entry =
                        library::find(id).unwrap_or_else(|| panic!("{}: unknown chord {id}", l.id));
                    let intro = c.order(entry.lesson).unwrap_or_else(|| {
                        panic!("{id} introduced in unknown lesson {}", entry.lesson)
                    });
                    assert!(
                        intro <= me,
                        "{}: asks for {id}, which isn't taught until {}",
                        l.id,
                        entry.lesson
                    );
                }
            }
        }
    }
}

#[test]
fn explanations_read_at_about_fifth_grade() {
    let c = curriculum();
    let mut too_hard = Vec::new();
    for l in c.lessons() {
        for s in &l.steps {
            if let Step::Learn { explain, title, .. } = s {
                let g = grade_level(explain);
                if g > 6.0 {
                    too_hard.push(format!("{} \"{title}\" reads at grade {g:.1}", l.id));
                }
            }
        }
    }
    assert!(
        too_hard.is_empty(),
        "too hard to read:\n{}",
        too_hard.join("\n")
    );
}

/// Every lesson teaches enough before it tests: several explained steps with
/// pictures, "watch out" tips, and a practice plan for the week.
#[test]
fn lessons_teach_in_depth() {
    let c = curriculum();
    let mut thin = Vec::new();
    for l in c.lessons() {
        let mut learns = 0;
        let mut words = 0;
        let mut has_tips = false;
        for s in &l.steps {
            if let Step::Learn {
                explain,
                facts,
                tips,
                ..
            } = s
            {
                learns += 1;
                words += explain.split_whitespace().count();
                words += facts
                    .iter()
                    .map(|f| f.split_whitespace().count())
                    .sum::<usize>();
                words += tips
                    .iter()
                    .map(|t| t.split_whitespace().count())
                    .sum::<usize>();
                has_tips |= !tips.is_empty();
            }
        }
        let visuals = l.steps.iter().flat_map(visuals_in).count();
        let mut why = Vec::new();
        if learns < 2 {
            why.push(format!("{learns} learn steps"));
        }
        if visuals < 2 {
            why.push(format!("{visuals} visuals"));
        }
        if words < 120 {
            why.push(format!("{words} teaching words"));
        }
        if !has_tips {
            why.push("no tips".into());
        }
        if l.practice.trim().is_empty() {
            why.push("no practice plan".into());
        }
        if !why.is_empty() {
            thin.push(format!("{}: {}", l.id, why.join(", ")));
        }
    }
    assert!(thin.is_empty(), "lessons too thin:\n{}", thin.join("\n"));
}

#[test]
fn placement_bank_is_sound() {
    let c = curriculum();
    let b = &c.placement;
    let mut ids = HashSet::new();
    for item in &b.items {
        assert!(ids.insert(&item.id), "duplicate placement id {}", item.id);
        assert!(item.level <= 7);
        check_question(&item.question, &item.id, false);
    }
    for level in 0..=7 {
        let n = b.items.iter().filter(|i| i.level == level).count();
        let threads: HashSet<_> = b
            .items
            .iter()
            .filter(|i| i.level == level)
            .map(|i| i.thread)
            .collect();
        assert!(
            n >= 4,
            "level {level} needs at least 4 checkpoint items (has {n})"
        );
        assert!(
            threads.len() >= 3,
            "level {level} needs items from at least 3 threads"
        );
    }
    for claim in &b.claims {
        assert!((1..=7).contains(&claim.level));
    }
}

#[test]
fn chord_name_normalizing() {
    assert_eq!(normalize_chord_name("E minor"), "Em");
    assert_eq!(normalize_chord_name("Easy C"), "C");
    assert_eq!(normalize_chord_name("B minor"), "Bm");
    assert_eq!(normalize_chord_name("D7"), "D7");
    assert_eq!(normalize_chord_name("E minor 7"), "Em7");
    assert_eq!(normalize_chord_name("F major 7"), "Fmaj7");
}
