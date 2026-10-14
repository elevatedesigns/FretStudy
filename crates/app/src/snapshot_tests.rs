//! Renders screens to PNGs (in `crates/app/tests/snapshots/`) so visual changes are
//! reviewed. Run `UPDATE_SNAPSHOTS=1 cargo test -p fretstudy` after intended changes.

use crate::app::{App, NeckView, Screen};
use crate::sound::Sound;
use crate::widgets::{ColorMode, LabelMode};
use egui_kittest::Harness;
use store::{AgeBand, Store};

/// An app with silent audio and an in-memory database, so images don't depend on the machine.
fn harness() -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 860.0))
        .build_eframe(|cc| App::with(cc, Sound::silent(), Store::open_in_memory().ok()));
    h.run();
    h
}

/// Same, with a profile signed in (placement done unless `fresh`).
fn signed_in(fresh: bool) -> Harness<'static, App> {
    let mut h = harness();
    let app = h.state_mut();
    let id = app
        .store
        .as_ref()
        .unwrap()
        .create_profile("Sam", "🐱", AgeBand::Kid, None)
        .unwrap();
    if !fresh {
        app.store
            .as_ref()
            .unwrap()
            .set_placement_done(id, true)
            .unwrap();
    }
    app.load_profile(id);
    h.run();
    h
}

#[test]
fn profiles() {
    let mut h = harness();
    h.snapshot("profiles_new_player");
}

#[test]
fn welcome() {
    let mut h = signed_in(true);
    h.snapshot("welcome");
}

#[test]
fn placement_screens() {
    let mut h = signed_in(true);
    h.state_mut().placement = Some(crate::screens::placement::PlacementUi::new());
    h.state_mut().screen = Screen::Placement;
    h.run();
    h.snapshot("placement_experience");
}

#[test]
fn path_screens() {
    let mut h = signed_in(false);
    h.state_mut().screen = Screen::Path;
    h.run();
    h.snapshot("path_new_learner");

    // After placing into Level 2: Levels 0–1 placed out.
    let app = h.state_mut();
    let ids: Vec<String> = app
        .curriculum
        .lessons()
        .filter(|l| l.id.starts_with("L0") || l.id.starts_with("L1"))
        .map(|l| l.id.clone())
        .collect();
    app.complete_placement(&ids);
    let pid = app.profile.as_ref().unwrap().id;
    app.store
        .as_ref()
        .unwrap()
        .set_status(pid, "L1.9", "completed", Some(1.0))
        .unwrap();
    app.reload_progress();
    h.run();
    h.snapshot("path_placed_into_level2");
}

/// Opens `lesson` at the step titled `step` (by title, so new content steps don't shift shots).
fn lesson_step(h: &mut Harness<'static, App>, lesson: &str, step: &str, name: &str) {
    let index = h
        .state()
        .curriculum
        .lesson(lesson)
        .unwrap()
        .steps
        .iter()
        .position(|s| s.title() == step)
        .unwrap_or_else(|| panic!("{lesson} has no step \"{step}\""));
    h.state_mut().open_lesson(lesson);
    h.state_mut().lesson.as_mut().unwrap().jump(index);
    h.run();
    h.snapshot(name);
}

#[test]
fn lesson_screens() {
    let mut h = signed_in(false);
    lesson_step(
        &mut h,
        "L0.1",
        "The parts of a guitar",
        "lesson_L0_1_guitar_parts",
    );
    lesson_step(&mut h, "L0.2", "Your fretting hand", "lesson_L0_2_hand");
    lesson_step(
        &mut h,
        "L0.3",
        "Strings have numbers and names",
        "lesson_L0_3_strings",
    );
    lesson_step(&mut h, "L0.6", "Tab is a treasure map", "lesson_L0_6_tab");
    lesson_step(
        &mut h,
        "L1.1",
        "The one-finger cousin",
        "lesson_L1_1_chord_pair",
    );
    lesson_step(
        &mut h,
        "L1.2",
        "One strum per click",
        "lesson_L1_2_strum_grid",
    );
    lesson_step(
        &mut h,
        "L1.4",
        "One-minute changes: E minor and easy G",
        "lesson_L1_4_change_trainer",
    );
    lesson_step(&mut h, "L1.7", "Ode to Joy", "lesson_L1_7_ode_to_joy");
    lesson_step(&mut h, "L1.1", "Check It", "lesson_L1_1_check");
}

#[test]
fn level_2_to_4_screens() {
    let mut h = signed_in(false);
    lesson_step(&mut h, "L2.8", "Only seven letters", "lesson_L2_8_alphabet");
    lesson_step(&mut h, "L2.9", "The smallest step", "lesson_L2_9_piano");
    lesson_step(
        &mut h,
        "L2.9",
        "Note hunt: strings 1 and 2",
        "lesson_L2_9_note_hunt",
    );
    lesson_step(&mut h, "L2.3", "Strum check", "lesson_L2_3_chord_check");
    lesson_step(
        &mut h,
        "L3.2",
        "Two players at once",
        "lesson_L3_2_boom_chuck",
    );
    lesson_step(&mut h, "L3.5", "Sound and silence", "lesson_L3_5_durations");
    lesson_step(
        &mut h,
        "L3.11",
        "Chord families",
        "lesson_L3_11_number_wheel",
    );
    lesson_step(&mut h, "L4.9", "A moving nut", "lesson_L4_9_capo");
    lesson_step(
        &mut h,
        "L4.10",
        "The Do-Re-Mi ladder",
        "lesson_L4_10_scale_steps",
    );
    lesson_step(&mut h, "L4.13", "A song in 6/8", "lesson_L4_13_arpeggio68");
}

#[test]
fn level_5_screens() {
    let mut h = signed_in(false);
    lesson_step(&mut h, "L5.1", "A five-note ladder", "lesson_L5_1_box1");
    lesson_step(
        &mut h,
        "L5.3",
        "A 12-bar song map",
        "lesson_L5_3_twelve_bar",
    );
    lesson_step(&mut h, "L5.4", "The boogie riff", "lesson_L5_4_boogie");
    lesson_step(&mut h, "L5.5", "Hammer it on", "lesson_L5_5_hammer_tab");
    lesson_step(&mut h, "L5.7", "Bend string 3", "lesson_L5_7_bend_check");
    lesson_step(
        &mut h,
        "L5.7",
        "Check your bend",
        "lesson_L5_7_watch_out_tips",
    );
    lesson_step(&mut h, "L5.8", "Steady wobble", "lesson_L5_8_vibrato_check");
    lesson_step(
        &mut h,
        "L5.9",
        "One crunchy extra note",
        "lesson_L5_9_blues_scale",
    );
    lesson_step(&mut h, "L5.10", "Solo time", "lesson_L5_10_jam");
    lesson_step(
        &mut h,
        "L5.11",
        "Major or minor 3rd?",
        "lesson_L5_11_ear_thirds",
    );
}

#[test]
fn level_6_screens() {
    let mut h = signed_in(false);
    lesson_step(
        &mut h,
        "L6.2",
        "Same box, sunny sound",
        "lesson_L6_2_major_pentatonic",
    );
    lesson_step(&mut h, "L6.4", "The whole map", "lesson_L6_4_whole_neck");
    lesson_step(
        &mut h,
        "L6.5",
        "Five shapes, one chord",
        "lesson_L6_5_caged",
    );
    lesson_step(
        &mut h,
        "L6.6",
        "The smallest real chord",
        "lesson_L6_6_triad",
    );
    lesson_step(&mut h, "L6.9", "A funk groove", "lesson_L6_9_funk16");
    lesson_step(
        &mut h,
        "L6.11",
        "A thumb that never stops",
        "lesson_L6_11_travis",
    );
}

#[test]
fn level_7_screens() {
    let mut h = signed_in(false);
    lesson_step(
        &mut h,
        "L7.1",
        "One more note on top",
        "lesson_L7_1_decoder",
    );
    lesson_step(
        &mut h,
        "L7.1",
        "The 7th chords of a key",
        "lesson_L7_1_seventh_wheel",
    );
    lesson_step(
        &mut h,
        "L7.2",
        "Root on string 6",
        "lesson_L7_2_root6_shapes",
    );
    lesson_step(
        &mut h,
        "L7.3",
        "Chord tones follow the music",
        "lesson_L7_3_chord_tones",
    );
    lesson_step(
        &mut h,
        "L7.4",
        "Both ladders at once",
        "lesson_L7_4_scale_compare",
    );
    lesson_step(
        &mut h,
        "L7.6",
        "The chords of A minor",
        "lesson_L7_6_minor_wheel",
    );
    lesson_step(&mut h, "L7.9", "Cases in G", "lesson_L7_9_detective");
    lesson_step(&mut h, "L7.10", "Three ways", "lesson_L7_10_expression");
}

#[test]
fn standard_notation() {
    let mut h = signed_in(false);
    h.state_mut().style.show_staff = true;
    lesson_step(&mut h, "L1.7", "Ode to Joy", "notation_L1_7_ode_to_joy");
    lesson_step(
        &mut h,
        "L6.7",
        "G major, two octaves",
        "notation_L6_7_g_major",
    );
}

#[test]
fn style_track_screens() {
    let mut h = signed_in(false);
    lesson_step(&mut h, "R.2", "Drop the low string", "track_R_2_drop_d");
    assert_eq!(
        h.state().tuning,
        theory::Tuning::drop_d(),
        "the lesson retunes the app"
    );
    lesson_step(&mut h, "C.8", "A famous minuet", "track_C_8_minuet");
    lesson_step(
        &mut h,
        "J.1",
        "The sound of jazz in three notes",
        "track_J_1_shells",
    );
    lesson_step(&mut h, "F.6", "The Brazilian groove", "track_F_6_bossa");
    // Leaving the lesson goes back to standard tuning.
    h.state_mut().screen = Screen::Chords;
    h.run();
    assert_eq!(h.state().tuning, theory::Tuning::standard());
    // The path lists the tracks under Level 8 once every core lesson is done.
    let app = h.state_mut();
    let pid = app.profile.as_ref().unwrap().id;
    let core: Vec<String> = app
        .curriculum
        .levels
        .iter()
        .filter(|l| l.track.is_none())
        .flat_map(|l| l.lessons.iter().map(|x| x.id.clone()))
        .collect();
    for id in &core {
        app.store
            .as_ref()
            .unwrap()
            .set_status(pid, id, "completed", Some(1.0))
            .unwrap();
    }
    app.reload_progress();
    app.screen = Screen::Path;
    h.run();
    h.snapshot("path_style_tracks");
}

#[test]
fn song_builder_screen() {
    let mut h = signed_in(false);
    let app = h.state_mut();
    app.songs.song.sections[0].bars = ["1", "5", "6", "4", "1", "5", "4", "1"]
        .map(String::from)
        .to_vec();
    app.songs.song.sections[1].bars = ["4", "1", "5", "6", "4", "b7", "5", "5"]
        .map(String::from)
        .to_vec();
    app.screen = Screen::Songs;
    h.run();
    h.snapshot("songs_builder");
}

#[test]
fn practice_screens() {
    let mut h = signed_in(false);
    let app = h.state_mut();
    let pid = app.profile.as_ref().unwrap().id;
    // A finished lesson whose review is due today, and some practice history.
    let store = app.store.as_ref().unwrap();
    store
        .set_status(pid, "L1.1", "completed", Some(1.0))
        .unwrap();
    store
        .set_status(pid, "L1.4", "completed", Some(1.0))
        .unwrap();
    store
        .schedule_review(pid, "L1.1", store::day_number() - 1)
        .unwrap();
    store.log_practice(pid, "2026-10-12", 900, None).unwrap();
    store.log_practice(pid, "2026-10-13", 600, None).unwrap();
    app.reload_progress();
    app.screen = Screen::Practice;
    h.run();
    h.snapshot("practice_today");
    h.state_mut().practice.tab = crate::screens::practice::Tab::Review;
    h.run();
    h.snapshot("practice_review");
    h.state_mut().practice.tab = crate::screens::practice::Tab::Metronome;
    h.run();
    h.snapshot("practice_metronome");
    h.state_mut().practice.tab = crate::screens::practice::Tab::Ear;
    h.run();
    h.snapshot("practice_ear_gym");
}

#[test]
fn chords_screen() {
    let mut h = signed_in(false);
    h.state_mut().screen = Screen::Chords;
    h.state_mut().chord_id = "C";
    h.run();
    h.snapshot("chords_c_fingers");

    h.state_mut().chord_id = "F";
    h.state_mut().style.color_mode = ColorMode::Role;
    h.state_mut().style.label_mode = LabelMode::Interval;
    h.run();
    h.snapshot("chords_f_barre_roles");

    h.state_mut().chord_id = "Bm";
    h.state_mut().style.label_mode = LabelMode::Note;
    h.state_mut().style.left_handed = true;
    h.run();
    h.snapshot("chords_bm_lefty_notes");
}

#[test]
fn neck_screen() {
    let mut h = signed_in(false);
    h.state_mut().screen = Screen::Neck;
    h.state_mut().neck.view = NeckView::Box(1);
    h.run();
    h.snapshot("neck_a_minor_pentatonic_box1");

    h.state_mut().neck.kind = theory::ScaleKind::Blues;
    h.state_mut().style.label_mode = LabelMode::Interval;
    h.run();
    h.snapshot("neck_a_blues_intervals");
}

#[test]
fn tuner_and_sound_check() {
    let mut h = signed_in(false);
    h.state_mut().screen = Screen::Tuner;
    h.run();
    h.snapshot("tuner");
    h.state_mut().screen = Screen::SoundCheck;
    h.run();
    h.snapshot("sound_check");
}

#[test]
fn tuner_needle_reading() {
    // Frame 1 installs fonts; later frames draw (fonts apply from the next frame on).
    let mut h = Harness::builder()
        .with_size(egui::vec2(460.0, 440.0))
        .build_ui_state(
            |ui, fonts_ready: &mut bool| {
                if !*fonts_ready {
                    crate::theme::install_fonts(ui.ctx());
                    crate::theme::apply_style(ui.ctx());
                    *fonts_ready = true;
                    return;
                }
                crate::widgets::needle::TunerNeedle {
                    cents: Some(-12.0),
                    label: "A".into(),
                    sublabel: "String 5 · hearing 109.2 Hz".into(),
                    size: 420.0,
                    live: true,
                }
                .show(ui);
            },
            false,
        );
    h.run();
    h.snapshot("tuner_needle_flat_a");
}

// ---------------------------------------------------------------- end-to-end flows

use egui_kittest::kittest::Queryable;

#[test]
fn new_learner_completes_first_lesson() {
    let mut h = signed_in(true);
    assert_eq!(h.state().screen, Screen::Welcome);
    h.get_by_label("Start at the beginning ▶").click();
    h.run();
    assert_eq!(h.state().screen, Screen::Path);

    h.get_by_label("Start lesson").click();
    h.run();
    assert_eq!(h.state().screen, Screen::Lesson);

    // Jump to Check It and answer every question correctly.
    h.get_by_label("✅ Check It").click();
    h.run();
    let lesson = h.state().curriculum.lesson("L0.1").unwrap().clone();
    let content::Step::Check { questions, .. } = lesson.steps.last().unwrap() else {
        panic!()
    };
    for (i, q) in questions.iter().enumerate() {
        h.get_by_label(&q.choices[q.answer]).click();
        h.run();
        let next = if i + 1 == questions.len() {
            "See my score"
        } else {
            "Next question ▶"
        };
        h.get_by_label(next).click();
        h.run();
    }
    let app = h.state();
    assert_eq!(app.progress.get("L0.1"), Some(&content::Done::Completed));
    let saved = app
        .store
        .as_ref()
        .unwrap()
        .progress(app.profile.as_ref().unwrap().id)
        .unwrap();
    assert_eq!(saved["L0.1"].status, "completed");
    // The next lesson is now unlocked and suggested.
    assert_eq!(app.curriculum.next_lesson(&app.progress), Some("L0.2"));
}

#[test]
fn placement_quiz_places_a_strong_player() {
    let mut h = signed_in(true);
    h.get_by_label("Take the quiz ▶").click();
    h.run();
    h.get_by_label("More than 2 years").click();
    h.run();
    for claim in [
        "Tune my guitar",
        "Play G, C, D, Em, and Am",
        "Read guitar tab",
        "Strum along with a song without stopping",
    ] {
        h.get_by_label(claim).click();
        h.run();
    }
    h.get_by_label("Continue ▶").click();
    h.run();
    // Answer as a learner who knows Levels 0–2 and nothing beyond.
    let mut guard = 0;
    loop {
        let item = {
            let app = h.state();
            let Some(p) = app.placement.as_ref() else {
                break;
            };
            p.current_item().cloned()
        };
        let Some(item) = item else { break };
        let label = if item.level < 3 {
            item.question.choices[item.question.answer].clone()
        } else {
            "I'm not sure".to_string()
        };
        h.get_by_label(&label).click();
        h.run();
        guard += 1;
        assert!(guard <= 12, "too many questions");
    }
    h.get_by_label("Go to My Path ▶").click();
    h.run();
    let app = h.state();
    assert_eq!(app.screen, Screen::Path);
    assert_eq!(app.progress.get("L1.9"), Some(&content::Done::PlacedOut));
    assert_eq!(app.progress.get("L0.1"), Some(&content::Done::PlacedOut));
    assert!(app.profile.as_ref().unwrap().placement_done);
}

#[test]
fn diagrams_are_described_and_arrow_keys_move_through_lessons() {
    use egui_kittest::kittest::Queryable;
    let mut h = signed_in(false);
    h.state_mut().open_lesson("L2.2");
    h.run();
    // Step 1 shows the G and C chords; screen readers hear what each diagram shows.
    h.get_by_label_contains("String 6: fret 3, finger 2 (G)");
    // The right arrow moves to the next step; the left arrow comes back.
    h.key_press(egui::Key::ArrowRight);
    h.run();
    h.get_by_label_contains("Step 2 of");
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    h.get_by_label_contains("Step 1 of");
}

#[test]
fn settings_screen() {
    let mut h = signed_in(false);
    h.state_mut().screen = Screen::Settings;
    h.run();
    h.snapshot("settings");
}

#[test]
fn chord_inversions_and_finder() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 1500.0))
        .build_eframe(|cc| App::with(cc, Sound::silent(), Store::open_in_memory().ok()));
    h.run();
    let app = h.state_mut();
    let id = app
        .store
        .as_ref()
        .unwrap()
        .create_profile("Sam", "🐱", AgeBand::Kid, None)
        .unwrap();
    app.store
        .as_ref()
        .unwrap()
        .set_placement_done(id, true)
        .unwrap();
    app.load_profile(id);
    app.screen = Screen::Chords;
    app.chord_id = "C";
    app.chord_view.ways = crate::screens::chords::Ways::Inversions;
    h.run();
    h.snapshot("chords_c_inversions");
    // The finder: any chord, e.g. A minor 11, up the neck.
    let app = h.state_mut();
    app.chord_view.finder = Some((9, theory::ChordQuality::Min11));
    app.chord_view.ways = crate::screens::chords::Ways::Neck;
    h.run();
    h.snapshot("chords_finder_am11_neck");
}
