//! Hands-on lesson activities. Activities never block progress: they give practice and
//! feedback, while the Check It step decides when a lesson is complete.

use super::visuals;
use crate::app::App;
use crate::screens::tuner::mic_controls;
use crate::theme;
use crate::widgets::chord_box::ChordBox;
use crate::widgets::guitar_parts::{GuitarParts, Part};
use crate::widgets::kit;
use crate::widgets::neck_map::NeckMap;
use crate::widgets::needle::TunerNeedle;
use crate::widgets::{Dot, DotShape, rhythm, tab_staff};
use audio::listen::{Heard, NoteListener, matches_note};
use audio::onset::OnsetDetector;
use content::{Activity, EarKind, StrumPattern, TabNote};
use egui::{RichText, Ui};
use std::time::Instant;
use theory::library;
use theory::{NoteName, Pitch, Spelling, string_index, string_number};

fn rng(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn seed_now() -> u64 {
    // Snapshot tests need the same "random" rounds every run.
    if cfg!(test) {
        return 0x9E37_79B9_7F4A_7C15;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(7)
        | 1
}

fn note_name(freq: f32) -> String {
    let (p, _) = Pitch::nearest(freq as f64);
    theme::glyphs(&NoteName::from_pc(p.pc(), Spelling::Sharps).pretty())
}

/// A small "🎤 the mic is listening" banner, or the mic controls if it's off.
fn mic_banner(app: &mut App, ui: &mut Ui, why: &str) -> bool {
    if app.mic.is_on() {
        ui.label(RichText::new(format!("🎤 Listening — {why}")).color(theme::GREEN));
        true
    } else {
        ui.label(RichText::new(format!("Turn on the microphone so FretStudy can {why}. (Optional — you can still practice without it.)")).color(theme::SOFT_INK));
        mic_controls(app, ui);
        false
    }
}

/// Keeps a note listener in sync with the microphone and returns what it heard this frame.
fn listen(app: &App, listener: &mut Option<NoteListener>) -> Vec<Heard> {
    listen_with(app, listener, false)
}

/// Like `listen`; `legato` also catches notes that start without a pick (hammer-ons…).
fn listen_with(app: &App, listener: &mut Option<NoteListener>, legato: bool) -> Vec<Heard> {
    if !app.mic.is_on() {
        *listener = None;
        return Vec::new();
    }
    let l = listener.get_or_insert_with(|| NoteListener::new(app.mic.sample_rate()));
    l.legato = legato;
    l.push(app.mic.frame_start, &app.mic.frame_samples)
}

pub enum State {
    Checklist(Vec<bool>),
    Parts(PartsRun),
    Strings(StringsRun),
    Tune(TuneRun),
    StringCheck(StringCheckRun),
    Strum(StrumRun),
    Changes(ChangesRun),
    Notes(NotesRun),
    Ear(EarRun),
    Tap(TapRun),
    Hunt(HuntRun),
    ChordCheck(ChordCheckRun),
    Bend(BendRun),
    Vibrato(VibratoRun),
    Jam(JamRun),
    Record(RecordRun),
    Expression(ExpressionRun),
    Detective(DetectiveRun),
    Builder,
}

impl State {
    pub fn new(activity: &Activity) -> State {
        match activity {
            Activity::Checklist { items } => State::Checklist(vec![false; items.len()]),
            Activity::PartsGame { .. } => State::Parts(PartsRun::new()),
            Activity::StringGame { .. } => State::Strings(StringsRun::new()),
            Activity::Tune => State::Tune(TuneRun {
                in_tune_since: [None; 6],
                done: [false; 6],
                self_confirmed: false,
            }),
            Activity::StringCheck { .. } => State::StringCheck(StringCheckRun {
                idx: 0,
                results: Vec::new(),
                msg: None,
                listener: None,
            }),
            Activity::StrumAlong { .. } => State::Strum(StrumRun {
                tempo: 0,
                guide: true,
                headphones: false,
                phase: StrumPhase::Idle,
            }),
            Activity::ChangeTrainer { .. } => State::Changes(ChangesRun {
                phase: ChangePhase::Idle,
                best: None,
                loaded: false,
            }),
            Activity::PlayNotes { .. } => State::Notes(NotesRun {
                idx: 0,
                msg: None,
                listener: None,
            }),
            Activity::EarQuiz { .. } => State::Ear(EarRun::new()),
            Activity::TapBeat { .. } => State::Tap(TapRun {
                phase: TapPhase::Idle,
            }),
            Activity::NoteHunt { .. } => State::Hunt(HuntRun::new()),
            Activity::ChordCheck { .. } => State::ChordCheck(ChordCheckRun {
                idx: 0,
                results: Vec::new(),
                msg: None,
                listener: None,
            }),
            Activity::BendCheck { .. } => State::Bend(BendRun::default()),
            Activity::VibratoCheck { .. } => State::Vibrato(VibratoRun::default()),
            Activity::Jam { .. } => State::Jam(JamRun::default()),
            Activity::Record { .. } => State::Record(RecordRun::default()),
            Activity::Expression { .. } => State::Expression(ExpressionRun::default()),
            Activity::Detective { .. } => State::Detective(DetectiveRun::new()),
            Activity::SongBuilder { .. } => State::Builder,
        }
    }

    /// Stop any sound this activity scheduled (leaving the step).
    pub fn leave(&mut self, app: &mut App) {
        if matches!(
            self,
            State::Strum(_)
                | State::Tap(_)
                | State::Notes(_)
                | State::Jam(_)
                | State::Record(_)
                | State::Detective(_)
        ) {
            app.sound.cancel_scheduled();
        }
        if let State::Strum(r) = self {
            r.phase = StrumPhase::Idle;
        }
        if let State::Tap(r) = self {
            r.phase = TapPhase::Idle;
        }
        if let State::Jam(r) = self {
            r.end_ns = None;
        }
        if let State::Record(r) = self {
            r.end_ns = None;
        }
        // Never keep recording after leaving the step.
        app.mic.recording = None;
    }
}

pub fn show(app: &mut App, ui: &mut Ui, activity: &Activity, state: &mut State) {
    match (activity, state) {
        (Activity::Checklist { items }, State::Checklist(checks)) => {
            for (item, done) in items.iter().zip(checks.iter_mut()) {
                ui.checkbox(done, RichText::new(item).size(18.0));
            }
            if checks.iter().all(|c| *c) {
                ui.label(
                    RichText::new("✔ Great posture!")
                        .color(theme::GREEN)
                        .size(18.0),
                );
            }
        }
        (Activity::PartsGame { rounds }, State::Parts(run)) => run.show(ui, *rounds),
        (Activity::StringGame { rounds }, State::Strings(run)) => run.show(app, ui, *rounds),
        (Activity::Tune, State::Tune(run)) => run.show(app, ui),
        (Activity::StringCheck { chord }, State::StringCheck(run)) => run.show(app, ui, chord),
        (
            Activity::StrumAlong {
                chords,
                beats_per_chord,
                pattern,
                tempos,
                repeats,
            },
            State::Strum(run),
        ) => run.show(
            app,
            ui,
            chords,
            *beats_per_chord,
            *pattern,
            tempos,
            *repeats,
        ),
        (Activity::ChangeTrainer { a, b, target }, State::Changes(run)) => {
            run.show(app, ui, a, b, *target)
        }
        (Activity::PlayNotes { notes, tempo, bar }, State::Notes(run)) => {
            run.show(app, ui, notes, *tempo, *bar)
        }
        (Activity::EarQuiz { kind, rounds }, State::Ear(run)) => run.show(app, ui, *kind, *rounds),
        (Activity::TapBeat { bpm, taps }, State::Tap(run)) => run.show(app, ui, *bpm, *taps),
        (
            Activity::NoteHunt {
                strings,
                max_fret,
                naturals_only,
                rounds,
            },
            State::Hunt(run),
        ) => run.show(app, ui, strings, *max_fret, *naturals_only, *rounds),
        (Activity::ChordCheck { chords }, State::ChordCheck(run)) => run.show(app, ui, chords),
        (
            Activity::BendCheck {
                string,
                fret,
                steps,
            },
            State::Bend(run),
        ) => run.show(app, ui, *string, *fret, *steps),
        (Activity::VibratoCheck { string, fret }, State::Vibrato(run)) => {
            run.show(app, ui, *string, *fret)
        }
        (
            Activity::Jam {
                key,
                scale,
                shape,
                chords,
                pattern,
                bpm,
                chord_tones,
            },
            State::Jam(run),
        ) => run.show(
            app,
            ui,
            key,
            scale,
            *shape,
            chords,
            *pattern,
            *bpm,
            *chord_tones,
        ),
        (
            Activity::Record {
                chords,
                pattern,
                bpm,
            },
            State::Record(run),
        ) => run.show(app, ui, chords, *pattern, *bpm),
        (Activity::Expression { notes }, State::Expression(run)) => run.show(app, ui, notes),
        (Activity::Detective { key, rounds }, State::Detective(run)) => {
            run.show(app, ui, key, *rounds)
        }
        (
            Activity::SongBuilder {
                verse_bars,
                chorus_bars,
            },
            State::Builder,
        ) => song_builder_task(app, ui, *verse_bars, *chorus_bars),
        _ => {
            ui.colored_label(theme::VERMILLION, "This activity couldn't load.");
        }
    }
}

// ---------------------------------------------------------------- guitar parts game

pub struct PartsRun {
    round: u8,
    target: Part,
    first_try: bool,
    score: u8,
    feedback: Option<(bool, String)>,
    /// The part just found (shown in green until the learner moves on).
    solved: Option<Part>,
    seed: u64,
}

impl PartsRun {
    fn new() -> PartsRun {
        let mut seed = seed_now();
        let target = Part::ALL[(rng(&mut seed) % Part::ALL.len() as u64) as usize];
        PartsRun {
            round: 0,
            target,
            first_try: true,
            score: 0,
            feedback: None,
            solved: None,
            seed,
        }
    }

    fn show(&mut self, ui: &mut Ui, rounds: u8) {
        if self.round >= rounds {
            ui.label(
                RichText::new(format!(
                    "🎉 You found {} of {} on the first try!",
                    self.score, rounds
                ))
                .font(theme::heading(22.0)),
            );
            if ui.button("Play again").clicked() {
                *self = PartsRun::new();
            }
            return;
        }
        ui.label(
            RichText::new(format!(
                "Round {} of {}: find the {}",
                self.round + 1,
                rounds,
                self.target.name().to_lowercase()
            ))
            .font(theme::heading(22.0)),
        );
        let highlight = self.solved;
        let mut g = GuitarParts::new(ui.available_width().min(760.0));
        g.highlight = highlight;
        g.highlight_color = theme::GREEN;
        let r = g.show(ui);
        // Keyboard and screen-reader friendly: pick the part from a list instead.
        let mut listed = None;
        egui::CollapsingHeader::new("Pick from a list instead").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for p in Part::ALL {
                    if ui.button(p.name()).clicked() {
                        listed = Some(p);
                    }
                }
            });
        });
        // Once a part is found, clicks wait until the learner moves on.
        let pick = if self.solved.is_some() {
            None
        } else {
            r.clicked.or(listed)
        };
        if let Some(p) = pick {
            if p == self.target {
                if self.first_try {
                    self.score += 1;
                }
                self.feedback = Some((
                    true,
                    format!(
                        "✔ Yes! That's the {}. {}",
                        p.name().to_lowercase(),
                        p.about()
                    ),
                ));
                self.solved = Some(p);
            } else {
                self.first_try = false;
                self.feedback = Some((
                    false,
                    format!("That's the {}. Try again!", p.name().to_lowercase()),
                ));
            }
        }
        if let Some((ok, text)) = &self.feedback {
            ui.label(RichText::new(text).size(18.0).color(if *ok {
                theme::GREEN
            } else {
                theme::VERMILLION
            }));
        }
        if self.solved.is_some() && kit::primary(ui, None, "Next part ▶").clicked() {
            // A fresh round: new target, nothing highlighted, no old message.
            self.round += 1;
            self.first_try = true;
            self.solved = None;
            self.feedback = None;
            let mut next = self.target;
            while next == self.target {
                next = Part::ALL[(rng(&mut self.seed) % Part::ALL.len() as u64) as usize];
            }
            self.target = next;
        }
    }
}

// ---------------------------------------------------------------- find-the-string game

pub struct StringsRun {
    round: u8,
    target: u8,
    score: u8,
    first_try: bool,
    feedback: Option<(bool, String)>,
    /// Found this round's string; waiting for Next.
    solved: bool,
    seed: u64,
    listener: Option<NoteListener>,
}

impl StringsRun {
    fn new() -> StringsRun {
        let mut seed = seed_now();
        let target = (rng(&mut seed) % 6) as u8 + 1;
        StringsRun {
            round: 0,
            target,
            score: 0,
            first_try: true,
            feedback: None,
            solved: false,
            seed,
            listener: None,
        }
    }

    fn answer(&mut self, number: u8) {
        if self.solved {
            return;
        }
        if number == self.target {
            if self.first_try {
                self.score += 1;
            }
            self.feedback = Some((
                true,
                format!(
                    "✔ Yes! String {number} is {}.",
                    ["E", "B", "G", "D", "A", "E"][number as usize - 1]
                ),
            ));
            self.solved = true;
        } else {
            self.first_try = false;
            self.feedback = Some((false, format!("That's string {number}. Try again!")));
        }
    }

    fn show(&mut self, app: &mut App, ui: &mut Ui, rounds: u8) {
        if self.round >= rounds {
            ui.label(
                RichText::new(format!("🎉 {} of {} on the first try!", self.score, rounds))
                    .font(theme::heading(22.0)),
            );
            if ui.button("Play again").clicked() {
                *self = StringsRun::new();
            }
            return;
        }
        ui.label(
            RichText::new(format!(
                "Round {} of {}: find string {}",
                self.round + 1,
                rounds,
                self.target
            ))
            .font(theme::heading(22.0)),
        );
        let r = NeckMap::new(&app.tuning, &[])
            .frets(0, 5)
            .left_handed(app.style.left_handed)
            .height(200.0)
            .glow(app.sound.glow())
            .show(ui);
        if let Some((s, _)) = r.clicked {
            app.sound.note(&app.tuning, s, 0);
            self.answer(string_number(s));
        }
        ui.label(
            RichText::new(
                "Click the string on the neck — or pluck it on your guitar with the mic on.",
            )
            .color(theme::SOFT_INK),
        );
        if app.mic.is_on() {
            for h in listen(app, &mut self.listener) {
                if let Heard::Note { freq, .. } = h
                    && let Some(s) = (0..6).find(|&s| {
                        matches_note(freq, app.tuning.open[s].0, 60.0)
                            && (1200.0 * (freq as f64 / app.tuning.open[s].frequency()).log2())
                                .abs()
                                < 80.0
                    })
                {
                    self.answer(string_number(s));
                }
            }
        }
        if let Some((ok, text)) = &self.feedback {
            ui.label(RichText::new(text).size(18.0).color(if *ok {
                theme::GREEN
            } else {
                theme::VERMILLION
            }));
        }
        if self.solved && kit::primary(ui, None, "Next string ▶").clicked() {
            self.round += 1;
            self.first_try = true;
            self.solved = false;
            self.feedback = None;
            let mut next = self.target;
            while next == self.target {
                next = (rng(&mut self.seed) % 6) as u8 + 1;
            }
            self.target = next;
        }
    }
}

// ---------------------------------------------------------------- tune all six

pub struct TuneRun {
    in_tune_since: [Option<Instant>; 6],
    done: [bool; 6],
    self_confirmed: bool,
}

impl TuneRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui) {
        mic_banner(app, ui, "hear each string");
        if app.mic.is_on() && app.tuner.is_none() {
            app.tuner = Some(audio::Tuner::new(app.mic.sample_rate()));
        }
        let names = app.tuning.open_names();
        let (reading, live) = app
            .tuner
            .as_ref()
            .map(|t| (t.reading(), t.is_live()))
            .unwrap_or((None, false));
        if let Some(r) = reading
            && let Some((s, c)) = r.string
        {
            if live && c.abs() <= 7.0 {
                let since = *self.in_tune_since[s].get_or_insert_with(Instant::now);
                if since.elapsed().as_secs_f32() > 0.4 {
                    self.done[s] = true;
                }
            } else {
                self.in_tune_since[s] = None;
            }
        }
        ui.horizontal_top(|ui| {
            let (label, sub, cents) = match reading.and_then(|r| r.string.map(|(s, c)| (r, s, c))) {
                Some((r, s, c)) => (
                    names[s].to_string(),
                    format!("String {} · {:.1} Hz", string_number(s), r.frequency),
                    Some(c),
                ),
                None => ("—".into(), "Pluck a string…".into(), None),
            };
            TunerNeedle {
                cents,
                label,
                sublabel: sub,
                size: 360.0,
                live,
            }
            .show(ui);
            ui.vertical(|ui| {
                for s in (0..6).rev() {
                    ui.horizontal(|ui| {
                        let mark = if self.done[s] {
                            RichText::new("✔").color(theme::GREEN).strong()
                        } else {
                            RichText::new("○").color(theme::SOFT_INK)
                        };
                        ui.label(mark.size(20.0));
                        ui.label(
                            RichText::new(format!("String {} · {}", string_number(s), names[s]))
                                .size(18.0),
                        );
                        if ui
                            .small_button("🔈")
                            .on_hover_text("Hear the in-tune note")
                            .clicked()
                        {
                            app.sound.note(&app.tuning, s, 0);
                        }
                    });
                }
            });
        });
        let count = self.done.iter().filter(|d| **d).count();
        if count == 6 || self.self_confirmed {
            ui.label(
                RichText::new("🎉 All six strings are in tune!")
                    .color(theme::GREEN)
                    .size(20.0),
            );
        } else {
            ui.label(RichText::new(format!("{count} of 6 strings tuned.")).size(17.0));
            if ui.button("I tuned with another tuner").clicked() {
                self.self_confirmed = true;
            }
        }
    }
}

// ---------------------------------------------------------------- chord string check

pub struct StringCheckRun {
    idx: usize,
    results: Vec<Option<bool>>,
    msg: Option<String>,
    listener: Option<NoteListener>,
}

impl StringCheckRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, chord_id: &str) {
        let Some(entry) = library::find(chord_id) else {
            return;
        };
        let shape = entry.shape();
        let chord = entry.chord();
        let played: Vec<usize> = (0..6).filter(|&s| shape.frets[s].is_some()).collect();
        if self.results.len() != played.len() {
            self.results = vec![None; played.len()];
        }
        let using_mic = mic_banner(app, ui, "check each string");
        ui.horizontal_top(|ui| {
            let mut glow = app.sound.glow();
            if let Some(&s) = played.get(self.idx) {
                glow[s] = glow[s].max(0.7);
            }
            ChordBox::new(&shape, Some(&chord), &app.tuning)
                .style(app.style)
                .width(240.0)
                .glow(glow)
                .show(ui);
            ui.vertical(|ui| {
                for (i, &s) in played.iter().enumerate() {
                    let fret = shape.frets[s].unwrap();
                    let (mark, color) = match self.results[i] {
                        Some(true) => ("✔ rings clearly", theme::GREEN),
                        Some(false) => ("✖ muted or buzzing", theme::VERMILLION),
                        None if i == self.idx => ("◀ pluck this one", theme::ORANGE),
                        None => ("", theme::SOFT_INK),
                    };
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "String {} ({})",
                                string_number(s),
                                if fret == 0 {
                                    "open".to_string()
                                } else {
                                    format!("fret {fret}")
                                }
                            ))
                            .size(17.0),
                        );
                        ui.label(RichText::new(mark).color(color).size(17.0));
                    });
                }
            });
        });
        if self.idx >= played.len() {
            let good = self.results.iter().filter(|r| **r == Some(true)).count();
            if good == played.len() {
                ui.label(
                    RichText::new("🎉 Every string rings clearly!")
                        .color(theme::GREEN)
                        .size(20.0),
                );
            } else {
                ui.label(RichText::new(format!("{good} of {} strings ring clearly. Check the Buzz Detective tips: fingertips, close to the fret, arch your fingers, press just enough.", played.len())).size(17.0));
            }
            if ui.button("Check again").clicked() {
                *self = StringCheckRun {
                    idx: 0,
                    results: Vec::new(),
                    msg: None,
                    listener: None,
                };
            }
            return;
        }
        let s = played[self.idx];
        let fret = shape.frets[s].unwrap();
        let expected = app.tuning.pitch(s, fret);
        for h in listen(app, &mut self.listener) {
            match h {
                Heard::Note { freq, .. } if matches_note(freq, expected.0, 40.0) => {
                    self.results[self.idx] = Some(true);
                    self.idx += 1;
                    self.msg = None;
                    break;
                }
                Heard::Note { freq, .. } => {
                    self.msg = Some(format!(
                        "I heard {} but expected {} on string {}. Is a finger touching it, or is the wrong string ringing?",
                        note_name(freq),
                        theme::glyphs(&NoteName::from_pc(expected.pc(), Spelling::Sharps).pretty()),
                        string_number(s)
                    ));
                }
                Heard::Unclear { .. } => {
                    self.msg = Some(format!(
                        "String {} didn't ring clearly. Make sure nothing is leaning on it.",
                        string_number(s)
                    ));
                }
            }
        }
        if let Some(m) = &self.msg {
            ui.label(RichText::new(m).color(theme::VERMILLION).size(16.0));
        }
        ui.horizontal(|ui| {
            if ui.button("🔈 Hear this string").clicked() {
                app.sound.note(&app.tuning, s, fret);
            }
            if using_mic {
                if ui.button("It won't ring — skip it").clicked() {
                    self.results[self.idx] = Some(false);
                    self.idx += 1;
                    self.msg = None;
                }
            } else {
                if ui.button("✔ It rings clearly").clicked() {
                    self.results[self.idx] = Some(true);
                    self.idx += 1;
                }
                if ui.button("✖ It's muted or buzzy").clicked() {
                    self.results[self.idx] = Some(false);
                    self.idx += 1;
                }
            }
        });
    }
}

// ---------------------------------------------------------------- strum along

pub enum StrumPhase {
    Idle,
    Running(Box<StrumLive>),
    Done { score: Option<(usize, usize)> },
}

pub struct StrumLive {
    beats_ns: Vec<u64>,
    count_in: usize,
    expected: Vec<u64>,
    hit: Vec<bool>,
    scoring: bool,
    onsets: OnsetDetector,
    end_ns: u64,
}

pub struct StrumRun {
    tempo: usize,
    guide: bool,
    headphones: bool,
    phase: StrumPhase,
}

impl StrumRun {
    #[allow(clippy::too_many_arguments)]
    fn show(
        &mut self,
        app: &mut App,
        ui: &mut Ui,
        chords: &[String],
        bpc: u8,
        pattern: StrumPattern,
        tempos: &[f32],
        repeats: u8,
    ) {
        let shapes: Vec<_> = chords.iter().filter_map(|id| library::find(id)).collect();
        if shapes.is_empty() || tempos.is_empty() {
            return;
        }
        let bpm = tempos[self.tempo.min(tempos.len() - 1)];
        let running = matches!(self.phase, StrumPhase::Running(_));
        if !running {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Speed:").color(theme::SOFT_INK));
                for (i, t) in tempos.iter().enumerate() {
                    ui.selectable_value(&mut self.tempo, i, format!("{t:.0} BPM"));
                }
                ui.add_space(10.0);
                ui.checkbox(&mut self.guide, "Play along with me");
                if app.mic.is_on() {
                    ui.checkbox(&mut self.headphones, "I'm wearing headphones");
                }
            });
            if app.mic.is_on() && !self.headphones {
                ui.label(RichText::new("🎤 The mic will check your timing. So it only hears you, the clicks stop after the count-in — follow the moving highlight.").small().color(theme::SOFT_INK));
            } else if !app.mic.is_on() {
                ui.label(RichText::new("Tip: turn on the microphone (Tuner or Sound Check) and FretStudy will check your timing.").small().color(theme::SOFT_INK));
            }
            let label = if matches!(self.phase, StrumPhase::Done { .. }) {
                "▶ Go again"
            } else {
                "▶ Start"
            };
            if ui.button(RichText::new(label).size(18.0)).clicked() {
                self.start(
                    app,
                    &shapes.iter().map(|e| e.shape()).collect::<Vec<_>>(),
                    bpc,
                    pattern,
                    bpm,
                    repeats,
                );
            }
        }

        // Live view
        let now = app.sound.now_ns();
        let mut finished = false;
        if let StrumPhase::Running(live) = &mut self.phase {
            if live.scoring
                && let Some(input) = app.mic.input.as_ref()
            {
                let latency_ns = (app.latency_ms.unwrap_or(0.0) * 1e6) as i64;
                for frame in live
                    .onsets
                    .push(app.mic.frame_start, &app.mic.frame_samples)
                {
                    let t = input.frame_captured_at(frame) as i64 - latency_ns;
                    if let Some((i, _)) = live
                        .expected
                        .iter()
                        .enumerate()
                        .filter(|(i, e)| !live.hit[*i] && (**e as i64 - t).abs() <= 110_000_000)
                        .min_by_key(|(_, e)| (**e as i64 - t).abs())
                    {
                        live.hit[i] = true;
                    }
                }
            }
            let beat = live.beats_ns.iter().rposition(|t| *t <= now);
            let beat_len = if live.beats_ns.len() > 1 {
                live.beats_ns[1] - live.beats_ns[0]
            } else {
                1
            };
            match beat {
                Some(b) if b < live.count_in => {
                    ui.label(
                        RichText::new(format!("Get ready… {}", b + 1))
                            .font(theme::heading(36.0))
                            .color(theme::VERMILLION),
                    );
                }
                Some(b) => {
                    let into = b - live.count_in;
                    let chord_i = (into / bpc as usize) % shapes.len();
                    let next_i = ((into / bpc as usize) + 1) % shapes.len();
                    ui.horizontal_top(|ui| {
                        let entry = shapes[chord_i];
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("Now: {}", entry.label))
                                    .font(theme::heading(26.0)),
                            );
                            ChordBox::new(&entry.shape(), Some(&entry.chord()), &app.tuning)
                                .style(app.style)
                                .width(220.0)
                                .glow(app.sound.glow())
                                .show(ui);
                        });
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("Next: {}", shapes[next_i].label))
                                    .color(theme::SOFT_INK)
                                    .size(18.0),
                            );
                            let spb = pattern.slots_per_beat();
                            let bpb = pattern.beats_per_bar();
                            let frac = (now - live.beats_ns[b]) as f32 / beat_len.max(1) as f32;
                            let part = (1..spb).filter(|k| pattern.slot_offset(*k) <= frac).count();
                            let slot = (b - live.count_in) % bpb * spb + part.min(spb - 1);
                            rhythm::strum_grid(ui, pattern, Some(slot), 420.0);
                            let flash = (1.0 - (now - live.beats_ns[b]) as f32 / 150_000_000.0)
                                .clamp(0.0, 1.0);
                            rhythm::beat_pulse(
                                ui,
                                bpb,
                                Some((b - live.count_in) % bpb),
                                flash,
                                260.0,
                            );
                        });
                    });
                }
                None => {
                    ui.label(RichText::new("Get ready…").font(theme::heading(36.0)));
                }
            }
            if ui.button("■ Stop").clicked() {
                app.sound.cancel_scheduled();
                finished = true;
            }
            if now > live.end_ns {
                finished = true;
            }
        }
        if finished && let StrumPhase::Running(live) = &self.phase {
            let score = live
                .scoring
                .then(|| (live.hit.iter().filter(|h| **h).count(), live.hit.len()));
            self.phase = StrumPhase::Done { score };
        }
        if let StrumPhase::Done { score } = &self.phase {
            match score {
                Some((hits, total)) => {
                    let pct = *hits as f32 / (*total).max(1) as f32 * 100.0;
                    let (msg, color) = if pct >= 80.0 {
                        ("🎉 Great timing!", theme::GREEN)
                    } else if pct >= 50.0 {
                        ("Good start! Try once more.", theme::ORANGE)
                    } else {
                        ("Keep practicing — try a slower speed.", theme::VERMILLION)
                    };
                    ui.label(
                        RichText::new(format!(
                            "{msg} You were on time for {hits} of {total} strums ({pct:.0}%)."
                        ))
                        .size(18.0)
                        .color(color),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("🎉 Nice strumming! Try the next speed when it feels easy.")
                            .size(18.0)
                            .color(theme::GREEN),
                    );
                }
            }
        }
    }

    fn start(
        &mut self,
        app: &mut App,
        shapes: &[theory::ChordShape],
        bpc: u8,
        pattern: StrumPattern,
        bpm: f32,
        repeats: u8,
    ) {
        let Some(start) = app.sound.frame_after(0.4) else {
            self.phase = StrumPhase::Done { score: None };
            return;
        };
        app.sound.cancel_scheduled();
        let scoring = app.mic.is_on();
        let quiet_clicks = scoring && !self.headphones;
        let guide = self.guide && !(scoring && !self.headphones);
        let beat = app.sound.frames_for(60.0 / bpm as f64);
        let spb = pattern.slots_per_beat();
        let bpb = pattern.beats_per_bar();
        let slots = pattern.slots();
        // Count in one full bar (4 clicks in 4/4, 6 in 6/8).
        let count_in = bpb;
        let playing_beats = shapes.len() * bpc as usize * repeats.max(1) as usize;
        let tuning = app.tuning.clone();
        let mut beats_ns = Vec::new();
        let mut expected = Vec::new();
        for b in 0..(count_in + playing_beats) {
            let frame = start + beat * b as u64;
            beats_ns.push(app.sound.heard_ns(frame));
            if b < count_in || !quiet_clicks {
                app.sound.click(frame, b % bpb == 0);
            }
            if b < count_in {
                continue;
            }
            let into = b - count_in;
            let shape = &shapes[(into / bpc as usize) % shapes.len()];
            for k in 0..spb {
                let stroke = slots[(into % bpb) * spb + k];
                let f = frame + (beat as f64 * pattern.slot_offset(k) as f64) as u64;
                if stroke.sounds() {
                    expected.push(app.sound.heard_ns(f));
                    if guide {
                        app.sound.schedule_stroke(shape, &tuning, f, stroke, 0.55);
                    }
                }
            }
        }
        let end_ns = *beats_ns.last().unwrap() + 700_000_000;
        let hit = vec![false; expected.len()];
        let mut onsets = OnsetDetector::new(app.mic.sample_rate());
        onsets.ratio = 4.0;
        self.phase = StrumPhase::Running(Box::new(StrumLive {
            beats_ns,
            count_in,
            expected,
            hit,
            scoring,
            onsets,
            end_ns,
        }));
    }
}

// ---------------------------------------------------------------- one-minute changes

pub enum ChangePhase {
    Idle,
    Running {
        end_ns: u64,
        count: u32,
        last_ns: u64,
        onsets: Box<OnsetDetector>,
    },
    Done {
        count: u32,
    },
}

pub struct ChangesRun {
    phase: ChangePhase,
    best: Option<u32>,
    loaded: bool,
}

impl ChangesRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, a: &str, b: &str, target: u32) {
        if !self.loaded {
            self.best = app.best_changes(a, b);
            self.loaded = true;
        }
        let (Some(ea), Some(eb)) = (library::find(a), library::find(b)) else {
            return;
        };
        let now = app.sound.now_ns().max(1);
        let mut count_now = 0;
        let mut finish = false;
        if let ChangePhase::Running {
            end_ns,
            count,
            last_ns,
            onsets,
        } = &mut self.phase
        {
            let add = |t: u64, count: &mut u32, last_ns: &mut u64| {
                if t.saturating_sub(*last_ns) > 350_000_000 {
                    *count += 1;
                    *last_ns = t;
                }
            };
            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                add(now, count, last_ns);
            }
            if let Some(input) = app.mic.input.as_ref() {
                for frame in onsets.push(app.mic.frame_start, &app.mic.frame_samples) {
                    add(input.frame_captured_at(frame), count, last_ns);
                }
            }
            count_now = *count;
            if now >= *end_ns {
                finish = true;
            }
        }
        let current_is_a = count_now % 2 == 0;
        ui.horizontal_top(|ui| {
            for (entry, is_now) in [(ea, current_is_a), (eb, !current_is_a)] {
                ui.vertical(|ui| {
                    let running = matches!(self.phase, ChangePhase::Running { .. });
                    let title = if running && is_now {
                        format!("▶ {}", entry.label)
                    } else {
                        entry.label.to_string()
                    };
                    ui.label(RichText::new(title).font(theme::heading(22.0)).color(
                        if running && is_now {
                            theme::VERMILLION
                        } else {
                            theme::INK
                        },
                    ));
                    ChordBox::new(&entry.shape(), Some(&entry.chord()), &app.tuning)
                        .style(app.style)
                        .width(200.0)
                        .show(ui);
                });
                ui.add_space(20.0);
            }
        });
        match &self.phase {
            ChangePhase::Idle | ChangePhase::Done { .. } => {
                if let ChangePhase::Done { count } = self.phase {
                    let msg = if count >= target {
                        format!("🎉 {count} changes! You beat the goal of {target}.")
                    } else {
                        format!(
                            "{count} changes. The goal is {target} — you'll get there with daily practice!"
                        )
                    };
                    ui.label(RichText::new(msg).size(19.0).color(if count >= target {
                        theme::GREEN
                    } else {
                        theme::ORANGE
                    }));
                }
                if let Some(best) = self.best {
                    ui.label(
                        RichText::new(format!("Your best: {best} changes in a minute"))
                            .color(theme::SOFT_INK),
                    );
                }
                ui.label(RichText::new("Strum once each time you land the next chord. The mic counts your strums — or press Space / the button each time.").color(theme::SOFT_INK));
                if ui
                    .button(RichText::new("▶ Start the minute").size(18.0))
                    .clicked()
                {
                    self.phase = ChangePhase::Running {
                        end_ns: now + 60_000_000_000,
                        count: 0,
                        last_ns: 0,
                        onsets: Box::new(OnsetDetector::new(app.mic.sample_rate())),
                    };
                }
            }
            ChangePhase::Running { end_ns, .. } => {
                let left = end_ns.saturating_sub(now) as f32 / 1e9;
                ui.add(egui::ProgressBar::new(left / 60.0).text(format!("{left:.0} seconds left")));
                ui.label(RichText::new(format!("{count_now} changes")).font(theme::heading(34.0)));
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✔ Changed!").size(20.0)).clicked()
                        && let ChangePhase::Running { count, last_ns, .. } = &mut self.phase
                    {
                        *count += 1;
                        *last_ns = now;
                    }
                    if ui.button("■ Stop").clicked() {
                        finish = true;
                    }
                });
            }
        }
        if finish && let ChangePhase::Running { count, .. } = self.phase {
            app.record_changes(a, b, count);
            self.best = Some(self.best.unwrap_or(0).max(count));
            self.phase = ChangePhase::Done { count };
        }
    }
}

// ---------------------------------------------------------------- play notes from tab

pub struct NotesRun {
    idx: usize,
    msg: Option<String>,
    listener: Option<NoteListener>,
}

impl NotesRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, notes: &[TabNote], tempo: f32, bar: u8) {
        let positions = visuals::tab_positions(notes);
        let playing = app.sound.sequence_index();
        let width = ui.available_width().min(720.0);
        let staff = app.style.show_staff.then_some((&app.tuning, None));
        tab_staff::notation_rows(
            ui,
            notes,
            playing.or(Some(self.idx)),
            self.idx,
            width,
            16,
            staff,
            bar,
        );
        if self.idx < positions.len() {
            let (s, f, _) = positions[self.idx];
            let name = NoteName::from_pc(app.tuning.pitch(s, f).pc(), Spelling::Sharps);
            let dots = [Dot {
                string: s,
                fret: f,
                fill: if f == 0 {
                    egui::Color32::WHITE
                } else {
                    theme::finger_color(f.min(4))
                },
                label: theme::glyphs(&name.pretty()),
                shape: DotShape::Circle,
                faint: false,
            }];
            ui.label(
                RichText::new(format!(
                    "Next: string {}, {} — {}",
                    string_number(s),
                    if f == 0 {
                        "open".to_string()
                    } else {
                        format!("fret {f}")
                    },
                    theme::glyphs(&name.pretty())
                ))
                .font(theme::heading(20.0)),
            );
            let lo = positions.iter().map(|p| p.1).min().unwrap_or(0);
            let hi = positions.iter().map(|p| p.1).max().unwrap_or(0);
            let (first, last) = if hi <= 5 {
                (0, 5)
            } else {
                (lo.saturating_sub(1), hi + 1)
            };
            NeckMap::new(&app.tuning, &dots)
                .frets(first, last)
                .left_handed(app.style.left_handed)
                .height(170.0)
                .glow(app.sound.glow())
                .show(ui);
            let expected = app.tuning.pitch(s, f).0;
            let legato = notes.iter().any(|n| n.tech.is_some());
            for h in listen_with(app, &mut self.listener, legato) {
                match h {
                    Heard::Note { freq, .. } if matches_note(freq, expected, 45.0) => {
                        self.idx += 1;
                        self.msg = None;
                        break;
                    }
                    Heard::Note { freq, .. } => {
                        self.msg = Some(format!(
                            "I heard {} — looking for {}.",
                            note_name(freq),
                            theme::glyphs(&name.pretty())
                        ));
                    }
                    Heard::Unclear { .. } => {}
                }
            }
        } else {
            ui.label(
                RichText::new("🎉 You played every note!")
                    .font(theme::heading(22.0))
                    .color(theme::GREEN),
            );
        }
        if let Some(m) = &self.msg {
            ui.label(RichText::new(m).color(theme::ORANGE).size(16.0));
        }
        ui.horizontal(|ui| {
            if ui.button("▶ Listen").clicked() {
                let tuning = app.tuning.clone();
                app.sound
                    .play_tab(&tuning, &visuals::tab_timed(notes), tempo);
            }
            if self.idx < positions.len() {
                if ui.button("🔈 This note").clicked() {
                    let (s, f, _) = positions[self.idx];
                    app.sound.note(&app.tuning, s, f);
                }
                if ui
                    .button("Next ▶")
                    .on_hover_text("Move on without the mic")
                    .clicked()
                {
                    self.idx += 1;
                }
            }
            if ui.button("↺ Start over").clicked() {
                self.idx = 0;
                self.msg = None;
            }
        });
        mic_banner(app, ui, "follow along as you play");
    }
}

// ---------------------------------------------------------------- ear games

struct EarRound {
    choices: Vec<&'static str>,
    answer: usize,
    notes: Vec<(usize, u8)>,
    chords: Vec<&'static str>,
    /// Free pitches: (MIDI, seconds from start).
    pitches: Vec<(i32, f64)>,
}

pub struct EarRun {
    round: u8,
    score: u8,
    seed: u64,
    current: Option<EarRound>,
    picked: Option<usize>,
}

impl EarRun {
    fn new() -> EarRun {
        EarRun {
            round: 0,
            score: 0,
            seed: seed_now(),
            current: None,
            picked: None,
        }
    }

    fn make(&mut self, kind: EarKind, tuning: &theory::Tuning) -> EarRound {
        match kind {
            EarKind::HighLow => {
                let pool: Vec<(usize, u8)> = (2..6)
                    .flat_map(|s| [0u8, 1, 2, 3].map(|f| (s, f)))
                    .collect();
                let a = pool[(rng(&mut self.seed) % pool.len() as u64) as usize];
                let mut b = a;
                while tuning.pitch(b.0, b.1) == tuning.pitch(a.0, a.1) {
                    b = pool[(rng(&mut self.seed) % pool.len() as u64) as usize];
                }
                let answer = if tuning.pitch(a.0, a.1) > tuning.pitch(b.0, b.1) {
                    0
                } else {
                    1
                };
                EarRound {
                    choices: vec!["The first", "The second"],
                    answer,
                    notes: vec![a, b],
                    chords: vec![],
                    pitches: vec![],
                }
            }
            EarKind::MajorMinor => {
                const POOL: [(&str, bool); 8] = [
                    ("E", true),
                    ("Em", false),
                    ("A", true),
                    ("Am", false),
                    ("D", true),
                    ("Dm", false),
                    ("C", true),
                    ("G", true),
                ];
                let (id, major) = POOL[(rng(&mut self.seed) % POOL.len() as u64) as usize];
                EarRound {
                    choices: vec!["Major (bright)", "Minor (dark)"],
                    answer: if major { 0 } else { 1 },
                    notes: vec![],
                    chords: vec![id],
                    pitches: vec![],
                }
            }
            EarKind::FourOrFive => {
                let five = rng(&mut self.seed).is_multiple_of(2);
                EarRound {
                    choices: vec![
                        "The 4 chord (a little trip)",
                        "The 5 chord (wants to go home)",
                    ],
                    answer: usize::from(five),
                    notes: vec![],
                    chords: vec!["G", if five { "D" } else { "C" }],
                    pitches: vec![],
                }
            }
            EarKind::HomeOrAway => {
                let away = rng(&mut self.seed).is_multiple_of(2);
                EarRound {
                    choices: vec!["Home (finished)", "Away (wants to move)"],
                    answer: usize::from(away),
                    notes: vec![],
                    chords: vec!["G-easy", if away { "D7" } else { "G-easy" }],
                    pitches: vec![],
                }
            }
            EarKind::Thirds | EarKind::FourthsFifths => {
                // Two notes one after the other, then together.
                let root = 50 + (rng(&mut self.seed) % 12) as i32;
                let second = rng(&mut self.seed).is_multiple_of(2);
                let (choices, gap) = if kind == EarKind::Thirds {
                    (
                        vec!["Major 3rd (bright)", "Minor 3rd (dark)"],
                        if second { 3 } else { 4 },
                    )
                } else {
                    (
                        vec![
                            "4th (\"Here Comes the Bride\")",
                            "5th (\"Twinkle Twinkle\")",
                        ],
                        if second { 7 } else { 5 },
                    )
                };
                EarRound {
                    choices,
                    answer: usize::from(second),
                    notes: vec![],
                    chords: vec![],
                    pitches: vec![
                        (root, 0.0),
                        (root + gap, 0.7),
                        (root, 1.5),
                        (root + gap, 1.5),
                    ],
                }
            }
            EarKind::Progressions => {
                const PROGS: [[&str; 4]; 3] = [
                    ["G", "C", "D", "G"],
                    ["G", "D", "Em", "C"],
                    ["Em", "C", "G", "D"],
                ];
                let i = (rng(&mut self.seed) % 3) as usize;
                EarRound {
                    choices: vec!["1 – 4 – 5 – 1", "1 – 5 – 6 – 4", "6 – 4 – 1 – 5"],
                    answer: i,
                    notes: vec![],
                    chords: PROGS[i].to_vec(),
                    pitches: vec![],
                }
            }
            EarKind::Sevenths => {
                let root = 48 + (rng(&mut self.seed) % 8) as i32;
                let dom = rng(&mut self.seed).is_multiple_of(2);
                let top = if dom { 10 } else { 11 };
                let tones = [0, 4, 7, top];
                let mut pitches: Vec<(i32, f64)> = tones
                    .iter()
                    .enumerate()
                    .map(|(i, t)| (root + t, i as f64 * 0.35))
                    .collect();
                pitches.extend(tones.iter().map(|t| (root + t, 1.6)));
                EarRound {
                    choices: vec!["Major 7 (dreamy)", "Dominant 7 (bluesy)"],
                    answer: usize::from(dom),
                    notes: vec![],
                    chords: vec![],
                    pitches,
                }
            }
            EarKind::DorianAeolian | EarKind::MixolydianIonian => {
                // An A drone, then the scale climbing over it.
                let second = rng(&mut self.seed).is_multiple_of(2);
                let (steps, choices): ([i32; 8], Vec<&'static str>) =
                    if kind == EarKind::DorianAeolian {
                        (
                            if second {
                                [0, 2, 3, 5, 7, 8, 10, 12]
                            } else {
                                [0, 2, 3, 5, 7, 9, 10, 12]
                            },
                            vec!["Dorian (cool, bright 6th)", "Aeolian (sad natural minor)"],
                        )
                    } else {
                        (
                            if second {
                                [0, 2, 4, 5, 7, 9, 11, 12]
                            } else {
                                [0, 2, 4, 5, 7, 9, 10, 12]
                            },
                            vec!["Mixolydian (bluesy ♭7)", "Ionian (plain major)"],
                        )
                    };
                let mut pitches = vec![(45, 0.0), (52, 0.0), (45, 2.0), (52, 2.0)];
                pitches.extend(
                    steps
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (57 + s, 0.3 + i as f64 * 0.4)),
                );
                EarRound {
                    choices,
                    answer: usize::from(second),
                    notes: vec![],
                    chords: vec![],
                    pitches,
                }
            }
            EarKind::MinorFive => {
                let major = rng(&mut self.seed).is_multiple_of(2);
                EarRound {
                    choices: vec!["Minor v (E minor)", "Major V (E major)"],
                    answer: usize::from(major),
                    notes: vec![],
                    chords: vec!["Am", if major { "E" } else { "Em" }, "Am"],
                    pitches: vec![],
                }
            }
            EarKind::StraightOrSwing => {
                let swung = rng(&mut self.seed).is_multiple_of(2);
                let beat = 0.5;
                let late = if swung { beat * 2.0 / 3.0 } else { beat / 2.0 };
                // A little walking line, two notes per beat.
                const LINE: [i32; 8] = [45, 49, 52, 54, 55, 54, 52, 49];
                let pitches = LINE
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        (
                            *m,
                            (i / 2) as f64 * beat + if i % 2 == 1 { late } else { 0.0 },
                        )
                    })
                    .collect();
                EarRound {
                    choices: vec!["Straight (even)", "Swung (long-short)"],
                    answer: usize::from(swung),
                    notes: vec![],
                    chords: vec![],
                    pitches,
                }
            }
        }
    }

    fn play(app: &mut App, r: &EarRound) {
        let tuning = app.tuning.clone();
        if !r.pitches.is_empty() {
            app.sound.play_pitches_at(&r.pitches);
        } else if !r.notes.is_empty() {
            let pos: Vec<(usize, u8, f32)> = r.notes.iter().map(|&(s, f)| (s, f, 1.0)).collect();
            app.sound.play_timed(&tuning, &pos, 60.0);
        } else {
            let shapes: Vec<_> = r
                .chords
                .iter()
                .filter_map(|id| library::find(id))
                .map(|e| e.shape())
                .collect();
            let gap = if shapes.len() > 2 { 1.0 } else { 1.4 };
            app.sound.chords(&tuning, &shapes, gap);
        }
    }

    fn show(&mut self, app: &mut App, ui: &mut Ui, kind: EarKind, rounds: u8) {
        if self.round >= rounds {
            ui.label(
                RichText::new(format!("🎉 {} of {} right!", self.score, rounds))
                    .font(theme::heading(24.0)),
            );
            if ui.button("Play again").clicked() {
                *self = EarRun::new();
            }
            return;
        }
        if self.current.is_none() {
            let r = self.make(kind, &app.tuning.clone());
            EarRun::play(app, &r);
            self.current = Some(r);
            self.picked = None;
        }
        let Some(r) = self.current.as_ref() else {
            return;
        };
        ui.label(
            RichText::new(format!("Round {} of {}", self.round + 1, rounds)).color(theme::SOFT_INK),
        );
        if kind == EarKind::FourOrFive {
            ui.label(RichText::new("First you'll hear G, the 1 chord. Then the music moves. Did it go to the 4 chord or the 5 chord? Listen to the bass.").size(17.0));
        }
        let intro = match kind {
            EarKind::Thirds => Some(
                "Two notes, one after the other, then together. Is the gap a major 3rd (bright, happy) or a minor 3rd (dark, sad)?",
            ),
            EarKind::FourthsFifths => Some(
                "Two notes, then both together. A 4th sounds like the start of \"Here Comes the Bride.\" A 5th sounds like the jump in \"Twinkle, Twinkle.\"",
            ),
            EarKind::Progressions => Some(
                "Four chords in the key of G. Which numbers did you hear? Listen for where the minor chord (Em, the 6) shows up.",
            ),
            EarKind::StraightOrSwing => Some(
                "Listen to the walking notes. Are the pairs even (straight), or long-short like a gallop (swung)?",
            ),
            EarKind::Sevenths => Some(
                "Four notes up, then all together. Is the top note a major 7th (dreamy) or a ♭7 (bluesy)?",
            ),
            EarKind::DorianAeolian => Some(
                "A low A hums while a minor scale climbs. Does the 6th note sound bright (Dorian) or sad (Aeolian)?",
            ),
            EarKind::MixolydianIonian => Some(
                "A low A hums while a major scale climbs. Does the 7th note sound bluesy (Mixolydian) or plain (Ionian)?",
            ),
            EarKind::MinorFive => Some(
                "A minor, then the 5 chord, then home. Was the 5 chord minor or major? Major pulls home harder.",
            ),
            _ => None,
        };
        if let Some(t) = intro {
            ui.label(RichText::new(t).size(17.0));
        }
        if kind == EarKind::HomeOrAway {
            ui.label(RichText::new("First you'll hear G (home). Then: does the second chord sound finished, or like it wants to move?").size(17.0));
        }
        let mut replay = false;
        let mut picked_now = None;
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("🔊 Play again").size(18.0))
                .clicked()
            {
                replay = true;
            }
            for (i, c) in r.choices.iter().enumerate() {
                if self.picked.is_none()
                    && ui
                        .add(
                            egui::Button::new(RichText::new(*c).size(18.0))
                                .min_size(egui::vec2(180.0, 40.0)),
                        )
                        .clicked()
                {
                    picked_now = Some(i);
                }
            }
        });
        if replay {
            EarRun::play(app, r);
        }
        if let Some(i) = picked_now {
            self.picked = Some(i);
            if i == r.answer {
                self.score += 1;
            }
        }
        if let (Some(p), Some(r)) = (self.picked, self.current.as_ref()) {
            let right = p == r.answer;
            let what = if !r.chords.is_empty() && r.chords.len() <= 2 && kind != EarKind::MinorFive
            {
                let id = r.chords.last().unwrap();
                format!(
                    " It was {}.",
                    library::find(id).map(|e| e.label).unwrap_or(id)
                )
            } else {
                String::new()
            };
            ui.label(
                RichText::new(if right {
                    format!("✔ Yes!{what}")
                } else {
                    format!("Not this time — it was \"{}\".{what}", r.choices[r.answer])
                })
                .size(18.0)
                .color(if right {
                    theme::GREEN
                } else {
                    theme::VERMILLION
                }),
            );
            if ui.button(RichText::new("Next ▶").size(18.0)).clicked() {
                self.round += 1;
                self.current = None;
            }
        }
    }
}

// ---------------------------------------------------------------- tap the beat

pub enum TapPhase {
    Idle,
    Running {
        beats: Vec<u64>,
        streak: u32,
        best: u32,
        last: Option<i64>,
    },
    Done {
        success: bool,
        best: u32,
    },
}

pub struct TapRun {
    phase: TapPhase,
}

impl TapRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, bpm: f32, taps: u32) {
        let now = app.sound.now_ns();
        match &mut self.phase {
            TapPhase::Idle | TapPhase::Done { .. } => {
                if let TapPhase::Done { success, best } = self.phase {
                    if success {
                        ui.label(
                            RichText::new(format!(
                                "🎉 {taps} good taps in a row! You can feel the beat."
                            ))
                            .size(19.0)
                            .color(theme::GREEN),
                        );
                    } else {
                        ui.label(RichText::new(format!("Your best streak was {best}. Try again — listen for the click and tap right with it.")).size(18.0).color(theme::ORANGE));
                    }
                }
                if ui
                    .button(RichText::new(format!("▶ Start ({bpm:.0} BPM)")).size(18.0))
                    .clicked()
                {
                    let beats = app.sound.start_metronome(bpm, 48);
                    self.phase = TapPhase::Running {
                        beats,
                        streak: 0,
                        best: 0,
                        last: None,
                    };
                }
            }
            TapPhase::Running {
                beats,
                streak,
                best,
                last,
            } => {
                let (beat, flash) = app.sound.metronome_beat().unzip();
                rhythm::beat_pulse(ui, 4, beat, flash.unwrap_or(0.0), 360.0);
                let tapped = ui
                    .add(
                        egui::Button::new(RichText::new("TAP").font(theme::heading(40.0)))
                            .min_size(egui::vec2(260.0, 110.0)),
                    )
                    .clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Space));
                if tapped
                    && let Some(nearest) =
                        beats.iter().min_by_key(|b| (**b as i64 - now as i64).abs())
                {
                    let off_ms = (now as i64 - *nearest as i64) / 1_000_000;
                    *last = Some(off_ms);
                    if off_ms.abs() <= 70 {
                        *streak += 1;
                        *best = (*best).max(*streak);
                    } else {
                        *streak = 0;
                    }
                }
                if let Some(off) = *last {
                    let (text, color) = if off.abs() <= 70 {
                        ("✔ On time!".to_string(), theme::GREEN)
                    } else if off < 0 {
                        (format!("A little early ({} ms)", -off), theme::ORANGE)
                    } else {
                        (format!("A little late ({off} ms)"), theme::ORANGE)
                    };
                    ui.label(RichText::new(text).size(19.0).color(color));
                }
                ui.label(
                    RichText::new(format!("Streak: {streak} of {taps}")).font(theme::heading(22.0)),
                );
                let over = beats.last().is_some_and(|l| now > *l + 1_000_000_000);
                if *streak >= taps {
                    let b = *best;
                    app.sound.cancel_scheduled();
                    self.phase = TapPhase::Done {
                        success: true,
                        best: b,
                    };
                } else if over || ui.button("■ Stop").clicked() {
                    let b = *best;
                    app.sound.cancel_scheduled();
                    self.phase = TapPhase::Done {
                        success: false,
                        best: b,
                    };
                }
            }
        }
    }
}

// ---------------------------------------------------------------- note hunt

pub struct HuntRun {
    round: u8,
    score: u8,
    first_try: bool,
    target: Option<u8>,
    found: Option<(usize, u8)>,
    feedback: Option<(bool, String)>,
    seed: u64,
    listener: Option<NoteListener>,
}

/// "C", or "C♯ / D♭" for notes with two names.
fn both_names(pc: u8) -> String {
    let sharp = NoteName::from_pc(pc, Spelling::Sharps);
    let flat = NoteName::from_pc(pc, Spelling::Flats);
    let s = if sharp == flat {
        sharp.pretty()
    } else {
        format!("{} / {}", sharp.pretty(), flat.pretty())
    };
    theme::glyphs(&s)
}

impl HuntRun {
    fn new() -> HuntRun {
        HuntRun {
            round: 0,
            score: 0,
            first_try: true,
            target: None,
            found: None,
            feedback: None,
            seed: seed_now(),
            listener: None,
        }
    }

    fn pick(&mut self, tuning: &theory::Tuning, strings: &[u8], max_fret: u8, naturals_only: bool) {
        let positions: Vec<u8> = strings
            .iter()
            .flat_map(|n| (0..=max_fret).map(move |f| (theory::string_index(*n), f)))
            .map(|(s, f)| tuning.pitch(s, f).pc())
            .filter(|pc| !naturals_only || [0u8, 2, 4, 5, 7, 9, 11].contains(pc))
            .collect();
        if positions.is_empty() {
            return;
        }
        let mut next = self.target;
        for _ in 0..20 {
            next = Some(positions[(rng(&mut self.seed) % positions.len() as u64) as usize]);
            if next != self.target {
                break;
            }
        }
        self.target = next;
        self.first_try = true;
    }

    fn answer(&mut self, correct: bool, pos: Option<(usize, u8)>, said: String) {
        if correct {
            if self.first_try {
                self.score += 1;
            }
            self.round += 1;
            self.found = pos;
            self.feedback = Some((true, format!("✔ Yes! {said}")));
            self.target = None;
        } else {
            self.first_try = false;
            self.feedback = Some((false, format!("{said} Try again!")));
        }
    }

    fn show(
        &mut self,
        app: &mut App,
        ui: &mut Ui,
        strings: &[u8],
        max_fret: u8,
        naturals_only: bool,
        rounds: u8,
    ) {
        if self.round >= rounds {
            ui.label(
                RichText::new(format!("🎉 {} of {} on the first try!", self.score, rounds))
                    .font(theme::heading(22.0)),
            );
            if ui.button("Play again").clicked() {
                *self = HuntRun::new();
            }
            return;
        }
        if self.target.is_none() {
            let tuning = app.tuning.clone();
            self.pick(&tuning, strings, max_fret, naturals_only);
        }
        let Some(target) = self.target else { return };
        let where_ = if strings.len() == 1 {
            format!("on string {}", strings[0])
        } else {
            format!(
                "on strings {}",
                strings
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        ui.label(
            RichText::new(format!(
                "Round {} of {}: find {} {where_} (frets 0–{max_fret})",
                self.round + 1,
                rounds,
                both_names(target)
            ))
            .font(theme::heading(22.0)),
        );
        let dots: Vec<Dot> = self
            .found
            .iter()
            .map(|&(s, f)| Dot {
                string: s,
                fret: f,
                fill: theme::GREEN,
                label: theme::glyphs(
                    &NoteName::from_pc(app.tuning.pitch(s, f).pc(), Spelling::Sharps).pretty(),
                ),
                shape: DotShape::Circle,
                faint: false,
            })
            .collect();
        let r = NeckMap::new(&app.tuning, &dots)
            .frets(0, max_fret.max(5))
            .left_handed(app.style.left_handed)
            .height(210.0)
            .glow(app.sound.glow())
            .show(ui);
        if let Some((s, f)) = r.clicked {
            app.sound.note(&app.tuning, s, f);
            let pc = app.tuning.pitch(s, f).pc();
            let on_allowed = strings.contains(&string_number(s)) && f <= max_fret;
            let name = both_names(pc);
            if pc == target && on_allowed {
                self.answer(
                    true,
                    Some((s, f)),
                    format!("String {}, fret {f} is {name}.", string_number(s)),
                );
            } else if pc == target {
                self.answer(false, None, format!("That's {name}, but look {where_}."));
            } else {
                self.answer(
                    false,
                    None,
                    format!("That's {name} (string {}, fret {f}).", string_number(s)),
                );
            }
        }
        if app.mic.is_on() {
            for h in listen(app, &mut self.listener) {
                if let Heard::Note { freq, .. } = h {
                    let (p, _) = Pitch::nearest(freq as f64);
                    if p.pc() == target {
                        self.answer(true, None, format!("I heard {}.", both_names(target)));
                    } else {
                        self.answer(false, None, format!("I heard {}.", both_names(p.pc())));
                    }
                }
            }
        }
        if let Some((ok, text)) = &self.feedback {
            ui.label(RichText::new(text).size(18.0).color(if *ok {
                theme::GREEN
            } else {
                theme::VERMILLION
            }));
        }
        ui.label(
            RichText::new(
                "Click the spot on the neck — or turn on the mic and play it on your guitar.",
            )
            .color(theme::SOFT_INK),
        );
    }
}

// ---------------------------------------------------------------- strummed chord check

pub struct ChordCheckRun {
    idx: usize,
    results: Vec<Option<bool>>,
    msg: Option<String>,
    listener: Option<audio::chord::StrumListener>,
}

impl ChordCheckRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, chords: &[String]) {
        let entries: Vec<_> = chords.iter().filter_map(|id| library::find(id)).collect();
        if self.results.len() != entries.len() {
            self.results = vec![None; entries.len()];
        }
        let using_mic = mic_banner(app, ui, "check every string of your strum");
        ui.horizontal_wrapped(|ui| {
            for (i, e) in entries.iter().enumerate() {
                let (mark, color) = match self.results[i] {
                    Some(true) => ("✔", theme::GREEN),
                    Some(false) => ("✖", theme::VERMILLION),
                    None if i == self.idx => ("▶", theme::ORANGE),
                    None => ("○", theme::SOFT_INK),
                };
                ui.label(
                    RichText::new(format!("{mark} {}", e.label))
                        .color(color)
                        .size(17.0),
                );
                ui.add_space(8.0);
            }
        });
        if self.idx >= entries.len() {
            let good = self.results.iter().filter(|r| **r == Some(true)).count();
            let all = good == entries.len();
            let msg = if all {
                "🎉 Every chord rang clean!".to_string()
            } else {
                format!(
                    "{good} of {} chords rang clean. Use the string-by-string check to find the buzzy string.",
                    entries.len()
                )
            };
            ui.label(RichText::new(msg).size(19.0).color(if all {
                theme::GREEN
            } else {
                theme::ORANGE
            }));
            if ui.button("Check again").clicked() {
                *self = ChordCheckRun {
                    idx: 0,
                    results: Vec::new(),
                    msg: None,
                    listener: None,
                };
            }
            return;
        }
        let entry = entries[self.idx];
        let shape = entry.shape();
        let pitches = shape.pitches(&app.tuning);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("Strum: {}", entry.label)).font(theme::heading(24.0)),
                );
                ChordBox::new(&shape, Some(&entry.chord()), &app.tuning)
                    .style(app.style)
                    .width(230.0)
                    .glow(app.sound.glow())
                    .show(ui);
            });
            ui.vertical(|ui| {
                ui.label(RichText::new("Hold the chord and strum once. Let it ring.").size(17.0));
                if let Some(m) = &self.msg {
                    ui.label(RichText::new(m).size(17.0).color(theme::VERMILLION));
                }
            });
        });
        if using_mic && app.mic.is_on() {
            let l = self
                .listener
                .get_or_insert_with(|| audio::chord::StrumListener::new(app.mic.sample_rate()));
            let expected: Vec<f32> = pitches.iter().map(|(_, p)| p.frequency() as f32).collect();
            for (_, peaks) in l.push(app.mic.frame_start, &app.mic.frame_samples) {
                let v = audio::chord::verify_shape(&peaks, &expected);
                if v.ok() {
                    self.results[self.idx] = Some(true);
                    self.idx += 1;
                    self.msg = None;
                    break;
                }
                let parts: Vec<String> = v
                    .missing
                    .iter()
                    .map(|&m| {
                        let (s, p) = pitches[m];
                        format!(
                            "string {} ({})",
                            string_number(s),
                            theme::glyphs(&NoteName::from_pc(p.pc(), Spelling::Sharps).pretty())
                        )
                    })
                    .collect();
                self.msg = Some(if !parts.is_empty() {
                    format!(
                        "I didn't hear {}. Make sure that string rings — check the fingers next to it.",
                        parts.join(" and ")
                    )
                } else {
                    format!(
                        "I hear a note that isn't in {}. Check that each finger is on the right string and fret.",
                        entry.label
                    )
                });
            }
        } else {
            self.listener = None;
        }
        ui.horizontal(|ui| {
            if ui.button("🔈 Hear it").clicked() {
                app.sound
                    .strum(&shape, &app.tuning, audio::play::StrumDirection::Down);
            }
            if using_mic {
                if ui.button("Skip this chord").clicked() {
                    self.results[self.idx] = Some(false);
                    self.idx += 1;
                    self.msg = None;
                }
            } else {
                if ui.button("✔ It sounds clean").clicked() {
                    self.results[self.idx] = Some(true);
                    self.idx += 1;
                }
                if ui.button("✖ Something buzzes").clicked() {
                    self.results[self.idx] = Some(false);
                    self.idx += 1;
                }
            }
        });
    }
}

// ---------------------------------------------------------------- bends

/// One fretted note on a small neck, with an arrow for a bend.
fn single_note_neck(app: &App, ui: &mut Ui, s: usize, f: u8, bend_to: Option<u8>) {
    let name = NoteName::from_pc(app.tuning.pitch(s, f).pc(), Spelling::Sharps);
    let mut dots = vec![Dot {
        string: s,
        fret: f,
        fill: theme::finger_color(3),
        label: theme::glyphs(&name.pretty()),
        shape: DotShape::Circle,
        faint: false,
    }];
    if let Some(t) = bend_to {
        let target = NoteName::from_pc(app.tuning.pitch(s, t).pc(), Spelling::Sharps);
        dots.push(Dot {
            string: s,
            fret: t,
            fill: theme::GREEN,
            label: theme::glyphs(&target.pretty()),
            shape: DotShape::Circle,
            faint: true,
        });
    }
    let hi = bend_to.unwrap_or(f).max(f);
    NeckMap::new(&app.tuning, &dots)
        .frets(f.saturating_sub(3), hi + 2)
        .left_handed(app.style.left_handed)
        .height(170.0)
        .glow(app.sound.glow())
        .show(ui);
}

#[derive(Default)]
pub struct BendRun {
    track: Option<audio::technique::PitchTrack>,
    judge: Option<audio::technique::BendJudge>,
    verdict: Option<audio::technique::BendVerdict>,
    tries: u32,
    good: u32,
}

impl BendRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, string: u8, fret: u8, steps: u8) {
        use audio::technique::{BendJudge, BendVerdict, PitchTrack};
        let s = string_index(string.clamp(1, 6));
        let start = app.tuning.pitch(s, fret);
        let target = app.tuning.pitch(s, fret + steps);
        let start_name = theme::glyphs(&NoteName::from_pc(start.pc(), Spelling::Sharps).pretty());
        let target_name = theme::glyphs(&NoteName::from_pc(target.pc(), Spelling::Sharps).pretty());
        let size = if steps == 1 {
            "half step"
        } else {
            "whole step"
        };
        ui.label(
            RichText::new(format!(
                "Pick string {string}, fret {fret} ({start_name}). Push the string up a {size} until it sounds like fret {} ({target_name}).",
                fret + steps
            ))
            .font(theme::heading(20.0)),
        );
        single_note_neck(app, ui, s, fret, Some(fret + steps));
        ui.horizontal(|ui| {
            if ui.button("🔈 Hear the bend").clicked() {
                let tuning = app.tuning.clone();
                app.sound.play_tab(
                    &tuning,
                    &[(s, fret, 2.0, Some(content::Tech::Bend(steps)))],
                    60.0,
                );
            }
            if ui.button("🔈 Hear the goal note").clicked() {
                app.sound.note(&app.tuning, s, fret + steps);
            }
        });
        let using_mic = mic_banner(app, ui, "measure how far you bend");
        if !using_mic {
            self.track = None;
            return;
        }
        let track = self
            .track
            .get_or_insert_with(|| PitchTrack::new(app.mic.sample_rate()));
        let readings = track.push(app.mic.frame_start, &app.mic.frame_samples);
        let judge = self.judge.get_or_insert_with(|| {
            BendJudge::new(start.frequency() as f32, target.frequency() as f32)
        });
        if self.verdict.is_none()
            && let Some(v) = judge.push(&readings)
        {
            self.tries += 1;
            if matches!(v, BendVerdict::InTune { .. }) {
                self.good += 1;
            }
            self.verdict = Some(v);
        }
        // Meter: how far up the string is pushed, against the goal.
        let goal = judge.goal_cents();
        let now = judge.current_cents().unwrap_or(0.0).max(0.0);
        bend_meter(ui, now, goal);
        match self.verdict {
            Some(BendVerdict::InTune { .. }) => {
                ui.label(
                    RichText::new(format!("✔ Right on {target_name}! Nice bend."))
                        .size(19.0)
                        .color(theme::GREEN),
                );
            }
            Some(BendVerdict::Flat { off_cents }) => {
                ui.label(RichText::new(format!("Almost — push a little farther ({:.0} cents short). Use more fingers behind the bending finger.", -off_cents)).size(18.0).color(theme::ORANGE));
            }
            Some(BendVerdict::Sharp { off_cents }) => {
                ui.label(RichText::new(format!("A bit too far ({off_cents:.0} cents sharp). Listen for the goal note and stop there.")).size(18.0).color(theme::ORANGE));
            }
            None => {
                ui.label(
                    RichText::new("Pick the note, then bend and hold it.").color(theme::SOFT_INK),
                );
            }
        }
        if self.verdict.is_some() {
            ui.label(
                RichText::new(format!("In tune: {} of {} tries", self.good, self.tries))
                    .color(theme::SOFT_INK),
            );
            if ui.button("Try again").clicked() {
                self.verdict = None;
                self.judge = None;
            }
        }
    }
}

/// A horizontal meter from 0 to past the goal, with the goal marked in green.
fn bend_meter(ui: &mut Ui, cents: f32, goal: f32) {
    use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
    let width = ui.available_width().min(520.0);
    let (resp, painter) = ui.allocate_painter(Vec2::new(width, 56.0), Sense::hover());
    let r = resp.rect;
    let max = goal * 1.4;
    let x_for = |c: f32| r.left() + (c / max).clamp(0.0, 1.0) * width;
    let bar = Rect::from_min_max(
        Pos2::new(r.left(), r.top() + 8.0),
        Pos2::new(r.right(), r.top() + 32.0),
    );
    painter.rect_filled(bar, CornerRadius::same(8), theme::PANEL);
    let zone = Rect::from_min_max(
        Pos2::new(x_for(goal - 25.0), bar.top()),
        Pos2::new(x_for(goal + 25.0), bar.bottom()),
    );
    painter.rect_filled(
        zone,
        CornerRadius::same(4),
        theme::GREEN.gamma_multiply(0.35),
    );
    let fill = Rect::from_min_max(bar.min, Pos2::new(x_for(cents), bar.bottom()));
    painter.rect_filled(fill, CornerRadius::same(8), theme::ORANGE);
    painter.line_segment(
        [
            Pos2::new(x_for(goal), bar.top() - 4.0),
            Pos2::new(x_for(goal), bar.bottom() + 4.0),
        ],
        Stroke::new(3.0, theme::GREEN),
    );
    painter.text(
        Pos2::new(r.left(), r.bottom() - 10.0),
        Align2::LEFT_CENTER,
        "fretted note",
        FontId::proportional(13.0),
        theme::SOFT_INK,
    );
    painter.text(
        Pos2::new(x_for(goal), r.bottom() - 10.0),
        Align2::CENTER_CENTER,
        "goal",
        FontId::proportional(13.0),
        theme::GREEN,
    );
}

// ---------------------------------------------------------------- vibrato

#[derive(Default)]
pub struct VibratoRun {
    track: Option<audio::technique::PitchTrack>,
    readings: Vec<(u64, f32)>,
    result: Option<audio::technique::VibratoReading>,
}

impl VibratoRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, string: u8, fret: u8) {
        use audio::technique::{PitchTrack, measure_vibrato};
        let s = string_index(string.clamp(1, 6));
        let pitch = app.tuning.pitch(s, fret);
        let name = theme::glyphs(&NoteName::from_pc(pitch.pc(), Spelling::Sharps).pretty());
        ui.label(
            RichText::new(format!(
                "Pick string {string}, fret {fret} ({name}) and hold it. Rock the string up and down a tiny bit, steady like a heartbeat."
            ))
            .font(theme::heading(20.0)),
        );
        single_note_neck(app, ui, s, fret, None);
        if ui.button("🔈 Hear vibrato").clicked() {
            let tuning = app.tuning.clone();
            app.sound.play_tab(
                &tuning,
                &[(s, fret, 3.0, Some(content::Tech::Vibrato))],
                60.0,
            );
        }
        if !mic_banner(app, ui, "measure your vibrato") {
            self.track = None;
            return;
        }
        let sr = app.mic.sample_rate();
        let track = self.track.get_or_insert_with(|| PitchTrack::new(sr));
        let expected = pitch.frequency() as f32;
        for (t, f) in track.push(app.mic.frame_start, &app.mic.frame_samples) {
            // Keep only readings near the note (octave errors folded back).
            let mut f = f;
            while f > expected * 1.5 {
                f /= 2.0;
            }
            while f < expected / 1.5 {
                f *= 2.0;
            }
            let cents = 1200.0 * (f / expected).log2();
            if cents.abs() > 150.0 {
                self.readings.clear();
                continue;
            }
            // A gap means the note stopped: start over.
            if self
                .readings
                .last()
                .is_some_and(|(lt, _)| t - lt > sr as u64 / 5)
            {
                self.readings.clear();
            }
            self.readings.push((t, f));
        }
        let span = match (self.readings.first(), self.readings.last()) {
            (Some(a), Some(b)) => (b.0 - a.0) as f32 / sr as f32,
            _ => 0.0,
        };
        if span >= 1.5 {
            self.result = measure_vibrato(&self.readings, sr);
            self.readings.clear();
        }
        wobble_line(ui, &self.readings, expected);
        if let Some(r) = self.result {
            let (msg, color) = if r.steady() {
                ("✔ Steady vibrato!", theme::GREEN)
            } else if r.width_cents < 10.0 {
                ("Wider — rock the string a little more.", theme::ORANGE)
            } else if r.width_cents > 120.0 {
                (
                    "Smaller — that's almost a bend. Keep the wobble tiny.",
                    theme::ORANGE,
                )
            } else if r.rate_hz < 3.0 {
                (
                    "A bit faster — aim for about 5 wobbles a second.",
                    theme::ORANGE,
                )
            } else {
                ("A bit slower and smoother.", theme::ORANGE)
            };
            ui.label(RichText::new(msg).size(19.0).color(color));
            ui.label(
                RichText::new(format!(
                    "{:.1} wobbles a second · {:.0} cents wide",
                    r.rate_hz, r.width_cents
                ))
                .color(theme::SOFT_INK),
            );
        } else {
            ui.label(RichText::new("Hold the note for about 2 seconds.").color(theme::SOFT_INK));
        }
    }
}

/// The pitch of a held note over time: a wavy line means vibrato.
fn wobble_line(ui: &mut Ui, readings: &[(u64, f32)], expected: f32) {
    use egui::{Pos2, Sense, Stroke, Vec2};
    let width = ui.available_width().min(520.0);
    let (resp, painter) = ui.allocate_painter(Vec2::new(width, 80.0), Sense::hover());
    let r = resp.rect;
    painter.rect_filled(r, egui::CornerRadius::same(8), theme::PANEL);
    painter.line_segment(
        [
            Pos2::new(r.left(), r.center().y),
            Pos2::new(r.right(), r.center().y),
        ],
        Stroke::new(1.0, theme::SOFT_INK.gamma_multiply(0.5)),
    );
    if readings.len() < 2 {
        return;
    }
    let first = readings[0].0;
    let span = (readings.last().unwrap().0 - first).max(1) as f32;
    let pts: Vec<Pos2> = readings
        .iter()
        .map(|(t, f)| {
            let c = (1200.0 * (f / expected).log2()).clamp(-100.0, 100.0);
            Pos2::new(
                r.left() + (*t - first) as f32 / span * width,
                r.center().y - c / 100.0 * 36.0,
            )
        })
        .collect();
    painter.add(egui::Shape::line(pts, Stroke::new(2.5, theme::BLUE)));
}

// ---------------------------------------------------------------- jam

#[derive(Default)]
pub struct JamRun {
    /// When the backing stops (None = not playing).
    pub end_ns: Option<u64>,
    bar_starts: Vec<u64>,
    listener: Option<NoteListener>,
    in_scale: u32,
    total: u32,
    last: Option<(usize, u8, bool)>,
    /// Notes played on beat 1: (chord tones, all).
    downbeats: (u32, u32),
}

impl JamRun {
    #[allow(clippy::too_many_arguments)]
    fn show(
        &mut self,
        app: &mut App,
        ui: &mut Ui,
        key: &str,
        scale: &str,
        shape: Option<u8>,
        chords: &[String],
        pattern: StrumPattern,
        bpm: f32,
        chord_tones: bool,
    ) {
        let (Ok(root), Some(kind)) = (key.parse::<NoteName>(), content::scale_kind(scale)) else {
            ui.colored_label(theme::VERMILLION, "This jam couldn't load.");
            return;
        };
        let shapes: Vec<_> = chords.iter().filter_map(|id| library::find(id)).collect();
        if shapes.is_empty() {
            return;
        }
        let now = app.sound.now_ns();
        let playing = self.end_ns.is_some_and(|e| now < e);
        let bar = if playing {
            self.bar_starts.iter().rposition(|t| *t <= now)
        } else {
            None
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Chords:").color(theme::SOFT_INK));
            for (i, e) in shapes.iter().enumerate() {
                let lit = bar.is_some_and(|b| b % shapes.len() == i);
                let text = RichText::new(e.label).size(18.0);
                ui.label(if lit {
                    text.background_color(theme::YELLOW).strong()
                } else {
                    text
                });
            }
        });
        // The box around the lit notes, so the whole shape fits.
        let (first, last) = match shape {
            Some(_) => {
                let dots = crate::widgets::theory_visuals::scale_dots(
                    &app.tuning,
                    root,
                    kind,
                    shape,
                    (0, 17),
                    app.style.label_mode,
                );
                let lit: Vec<u8> = dots.iter().filter(|d| !d.faint).map(|d| d.fret).collect();
                let lo = lit.iter().copied().min().unwrap_or(0);
                let hi = lit.iter().copied().max().unwrap_or(12);
                (lo.saturating_sub(1), (hi + 1).max(lo + 5))
            }
            None => (0, 12),
        };
        let mut dots = crate::widgets::theory_visuals::scale_dots(
            &app.tuning,
            root,
            kind,
            shape,
            (first, last),
            app.style.label_mode,
        );
        // Chord-tone view: the current chord's notes in color, the rest of the scale gray.
        let current = shapes[bar.unwrap_or(0) % shapes.len()].chord();
        if chord_tones {
            for d in dots.iter_mut() {
                let pc = app.tuning.pitch(d.string, d.fret).pc();
                match current.role_of(pc) {
                    Some(role) => {
                        d.fill = theme::role_color(role);
                        d.label = theme::glyphs(
                            &current
                                .interval_of(pc)
                                .map(|i| i.label())
                                .unwrap_or_default(),
                        );
                        d.shape = if role == theory::Role::Root {
                            crate::widgets::DotShape::Square
                        } else {
                            crate::widgets::DotShape::Circle
                        };
                    }
                    None => {
                        d.fill = theme::LIGHT_GRAY;
                        d.shape = crate::widgets::DotShape::Circle;
                    }
                }
            }
            ui.label(
                RichText::new(theme::glyphs(&format!(
                    "Now: {} — colored dots are its chord tones (R, 3, 5, 7). Gray dots are the rest of the scale.",
                    current.pretty()
                )))
                .color(theme::SOFT_INK),
            );
        }
        let highlight = self.last.map(|(s, f, _)| (s, f));
        let r = NeckMap::new(&app.tuning, &dots)
            .frets(first, last)
            .left_handed(app.style.left_handed)
            .height(210.0)
            .highlight(highlight)
            .glow(app.sound.glow())
            .show(ui);
        if let Some((s, f)) = r.clicked {
            app.sound.note(&app.tuning, s, f);
        }
        ui.horizontal(|ui| {
            if !playing {
                if ui
                    .button(RichText::new(format!("▶ Start the backing ({bpm:.0} BPM)")).size(18.0))
                    .clicked()
                {
                    // Four times through the chords.
                    let bars: Vec<_> = (0..4)
                        .flat_map(|_| shapes.iter().map(|e| e.shape()))
                        .collect();
                    app.sound.cancel_scheduled();
                    if let Some((starts, end)) =
                        visuals::schedule_groove(app, &bars, pattern, bpm, true, false)
                    {
                        self.bar_starts = starts;
                        self.end_ns = Some(end);
                        self.in_scale = 0;
                        self.total = 0;
                        self.last = None;
                        self.downbeats = (0, 0);
                    }
                }
            } else if ui.button(RichText::new("■ Stop").size(18.0)).clicked() {
                app.sound.cancel_scheduled();
                self.end_ns = None;
            }
        });
        if !mic_banner(app, ui, "light up the notes you play") {
            return;
        }
        let scale_obj = theory::Scale::new(root, kind);
        let latency_ns = (app.latency_ms.unwrap_or(0.0) * 1e6) as i64;
        for h in listen_with(app, &mut self.listener, true) {
            if let Heard::Note { freq, onset } = h {
                let (p, _) = Pitch::nearest(freq as f64);
                let ok = scale_obj.contains(p.pc());
                // A note right on beat 1 of a bar: is it a tone of that bar's chord?
                if chord_tones
                    && playing
                    && let Some(input) = app.mic.input.as_ref()
                {
                    let t = input.frame_captured_at(onset) as i64 - latency_ns;
                    if let Some(b) = self.bar_starts.iter().position(|s| {
                        (t - *s as i64) > -120_000_000 && (t - *s as i64) < 250_000_000
                    }) {
                        let chord = shapes[b % shapes.len()].chord();
                        self.downbeats.1 += 1;
                        if chord.role_of(p.pc()).is_some() {
                            self.downbeats.0 += 1;
                        }
                    }
                }
                if playing {
                    self.total += 1;
                    if ok {
                        self.in_scale += 1;
                    }
                }
                // Light the spot inside the shown frets that makes this pitch.
                let pos = (0..6)
                    .filter_map(|s| {
                        let f = p.0 - app.tuning.open[s].0;
                        (f >= first as i32 && f <= last as i32).then_some((s, f as u8))
                    })
                    .min_by_key(|(s, _)| 5 - *s);
                if let Some((s, f)) = pos {
                    self.last = Some((s, f, ok));
                }
            }
        }
        if let Some((_, _, ok)) = self.last {
            ui.label(
                RichText::new(if ok {
                    "✔ That note is in the scale."
                } else {
                    "That one's outside the scale — slide to a dot next to it."
                })
                .color(if ok { theme::GREEN } else { theme::ORANGE }),
            );
        }
        if chord_tones && self.downbeats.1 > 0 {
            ui.label(
                RichText::new(format!(
                    "Beat 1 landed on a chord tone {} of {} times",
                    self.downbeats.0, self.downbeats.1
                ))
                .size(18.0)
                .color(theme::GREEN),
            );
        }
        if let Some(pct) = (self.in_scale * 100).checked_div(self.total) {
            ui.label(
                RichText::new(format!("{} notes played · {pct}% in the scale", self.total))
                    .size(18.0),
            );
            if !playing && pct >= 80 {
                ui.label(
                    RichText::new("🎉 Great jam! You stayed on the dots.")
                        .size(19.0)
                        .color(theme::GREEN),
                );
            }
        }
        if playing {
            ui.ctx().request_repaint();
        }
    }
}

// ---------------------------------------------------------------- record yourself

#[derive(Default)]
pub struct RecordRun {
    recorder: crate::recorder::RecorderUi,
    /// When the backing groove stops (None = not playing).
    pub end_ns: Option<u64>,
}

impl RecordRun {
    fn show(
        &mut self,
        app: &mut App,
        ui: &mut Ui,
        chords: &[String],
        pattern: StrumPattern,
        bpm: f32,
    ) {
        let lesson = app.lesson.as_ref().map(|l| l.lesson.id.clone());
        let shapes: Vec<_> = chords
            .iter()
            .filter_map(|id| library::find(id).map(|e| e.shape()))
            .collect();
        if !shapes.is_empty() {
            let playing = self.end_ns.is_some_and(|e| app.sound.now_ns() < e);
            ui.horizontal(|ui| {
                if !playing {
                    if ui
                        .button(
                            RichText::new(format!("▶ Start the backing ({bpm:.0} BPM)")).size(18.0),
                        )
                        .clicked()
                    {
                        let bars: Vec<_> = (0..4).flat_map(|_| shapes.iter().cloned()).collect();
                        app.sound.cancel_scheduled();
                        self.end_ns =
                            visuals::schedule_groove(app, &bars, pattern, bpm, true, true)
                                .map(|(_, end)| end);
                    }
                } else if ui
                    .button(RichText::new("■ Stop the backing").size(18.0))
                    .clicked()
                {
                    app.sound.cancel_scheduled();
                    self.end_ns = None;
                }
            });
            if playing {
                ui.ctx().request_repaint();
            }
            ui.label(
                RichText::new("Tip: wear headphones so the mic only hears your guitar.")
                    .small()
                    .color(theme::SOFT_INK),
            );
        }
        self.recorder.panel(app, ui, lesson.as_deref(), true, true);
    }
}

// ---------------------------------------------------------------- expression

#[derive(Clone, Copy, PartialEq, Eq)]
enum Way {
    Soft,
    Loud,
    Short,
}

impl Way {
    const ALL: [Way; 3] = [Way::Soft, Way::Loud, Way::Short];
    fn name(self) -> &'static str {
        match self {
            Way::Soft => "Soft (p)",
            Way::Loud => "Loud (f)",
            Way::Short => "Short and loud (staccato)",
        }
    }
}

#[derive(Default)]
pub struct ExpressionRun {
    takes: [Option<audio::dynamics::TakeStats>; 3],
    recording: Option<usize>,
    error: Option<String>,
}

impl ExpressionRun {
    fn show(&mut self, app: &mut App, ui: &mut Ui, notes: &[TabNote]) {
        let width = ui.available_width().min(720.0);
        let staff = app.style.show_staff.then_some((&app.tuning, None));
        tab_staff::notation_rows(
            ui,
            notes,
            app.sound.sequence_index(),
            0,
            width,
            16,
            staff,
            4,
        );
        if ui.button("▶ Listen").clicked() {
            let tuning = app.tuning.clone();
            app.sound
                .play_tab(&tuning, &visuals::tab_timed(notes), 80.0);
        }
        if !mic_banner(app, ui, "compare your three takes") {
            return;
        }
        let rate = app.mic.sample_rate();
        for (i, way) in Way::ALL.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(way.name()).size(18.0));
                if self.recording == Some(i) {
                    let secs =
                        app.mic.recording.as_ref().map_or(0, |r| r.len()) as f32 / rate as f32;
                    ui.label(RichText::new(format!("⏺ {secs:.0}s")).color(theme::VERMILLION));
                    ui.ctx().request_repaint();
                    if ui.button("■ Done").clicked()
                        && let Some(samples) = app.mic.recording.take()
                    {
                        self.recording = None;
                        self.takes[i] = audio::dynamics::measure(&samples, rate);
                        if self.takes[i].is_none() {
                            self.error = Some(
                                "I couldn't hear that take. Try again a bit closer to the mic."
                                    .into(),
                            );
                        } else {
                            self.error = None;
                        }
                    }
                } else if self.recording.is_none()
                    && ui
                        .button(if self.takes[i].is_some() {
                            "⏺ Again"
                        } else {
                            "⏺ Record"
                        })
                        .clicked()
                {
                    app.mic.recording = Some(Vec::new());
                    self.recording = Some(i);
                }
                if let Some(t) = self.takes[i] {
                    ui.label(
                        RichText::new(format!(
                            "loudness {:.0} dB · rings {:.0}% of the time",
                            t.attack_db,
                            t.sustain * 100.0
                        ))
                        .color(theme::SOFT_INK),
                    );
                }
            });
        }
        if let Some(e) = &self.error {
            ui.colored_label(theme::ORANGE, e);
        }
        if let [Some(soft), Some(loud), short] = self.takes {
            let gap = loud.attack_db - soft.attack_db;
            let (msg, good) = if gap >= 6.0 {
                (
                    format!(
                        "✔ Your loud take was {gap:.0} dB louder than your soft take. Big contrast!"
                    ),
                    true,
                )
            } else {
                (
                    format!(
                        "Your loud and soft takes are only {gap:.0} dB apart. Try softer, then harder."
                    ),
                    false,
                )
            };
            ui.label(RichText::new(msg).size(18.0).color(if good {
                theme::GREEN
            } else {
                theme::ORANGE
            }));
            if let Some(short) = short {
                let (msg, good) = if loud.sustain - short.sustain >= 0.2 {
                    ("✔ Your short take really was short. Nice staccato!", true)
                } else {
                    (
                        "Make the short take shorter: lift your fretting fingers right after each note.",
                        false,
                    )
                };
                ui.label(RichText::new(msg).size(18.0).color(if good {
                    theme::GREEN
                } else {
                    theme::ORANGE
                }));
            }
        }
    }
}

// ---------------------------------------------------------------- song detective

pub struct DetectiveRun {
    seed: u64,
    round: u8,
    score: u8,
    answer: Vec<usize>,
    guess: Vec<usize>,
    slow: bool,
}

impl DetectiveRun {
    fn new() -> DetectiveRun {
        DetectiveRun {
            seed: seed_now(),
            round: 0,
            score: 0,
            answer: Vec::new(),
            guess: Vec::new(),
            slow: false,
        }
    }

    fn play(&self, app: &mut App, key: NoteName) {
        let chords = crate::widgets::theory_visuals::key_chords(key, false, false);
        let shapes: Vec<_> = self
            .answer
            .iter()
            .filter_map(|i| library::shape_for(&chords[*i].chord, &app.tuning))
            .collect();
        let tuning = app.tuning.clone();
        app.sound
            .chords(&tuning, &shapes, if self.slow { 2.2 } else { 1.3 });
    }

    fn show(&mut self, app: &mut App, ui: &mut Ui, key: &str, rounds: u8) {
        let Ok(key) = key.parse::<NoteName>() else {
            return;
        };
        if self.round >= rounds {
            ui.label(
                RichText::new(format!(
                    "🎉 You cracked {} of {} progressions!",
                    self.score, rounds
                ))
                .font(theme::heading(24.0)),
            );
            if ui.button("Play again").clicked() {
                *self = DetectiveRun::new();
            }
            return;
        }
        if self.answer.is_empty() {
            // Always starts on the 1 chord; the rest come from the 2, 4, 5, and 6 chords.
            const POOL: [usize; 4] = [1, 3, 4, 5];
            self.answer = vec![0];
            for _ in 0..3 {
                self.answer.push(POOL[(rng(&mut self.seed) % 4) as usize]);
            }
            self.guess.clear();
            self.play(app, key);
        }
        ui.label(
            RichText::new(format!(
                "Case {} of {}: four chords in the key of {}. Tap their numbers in order.",
                self.round + 1,
                rounds,
                theme::glyphs(&key.pretty())
            ))
            .size(18.0),
        );
        ui.horizontal(|ui| {
            if ui.button("🔊 Play again").clicked() {
                self.play(app, key);
            }
            ui.checkbox(&mut self.slow, "Slow it down");
        });
        let chords = crate::widgets::theory_visuals::key_chords(key, false, false);
        let done = self.guess.len() == self.answer.len();
        if !done
            && let Some(i) = crate::widgets::theory_visuals::number_wheel(
                ui,
                &chords,
                ui.available_width().min(640.0),
            )
        {
            self.guess.push(i);
            if let Some(shape) = library::shape_for(&chords[i].chord, &app.tuning) {
                app.sound
                    .strum(&shape, &app.tuning, audio::play::StrumDirection::Down);
            }
            if self.guess == self.answer {
                self.score += 1;
            }
        }
        let shown: Vec<String> = self.guess.iter().map(|i| (i + 1).to_string()).collect();
        ui.label(
            RichText::new(format!("Your answer: {}", shown.join(" – "))).font(theme::heading(20.0)),
        );
        if !done && !self.guess.is_empty() && ui.button("↺ Clear").clicked() {
            self.guess.clear();
        }
        if done {
            let right = self.guess == self.answer;
            let real: Vec<String> = self
                .answer
                .iter()
                .map(|i| format!("{} ({})", i + 1, chords[*i].chord.pretty()))
                .collect();
            ui.label(
                RichText::new(if right {
                    format!("✔ Case closed! {}", real.join(" – "))
                } else {
                    format!(
                        "Not yet. It was {}. Listen again and follow the bass.",
                        real.join(" – ")
                    )
                })
                .size(18.0)
                .color(if right { theme::GREEN } else { theme::ORANGE }),
            );
            if ui.button(RichText::new("Next case ▶").size(18.0)).clicked() {
                self.round += 1;
                self.answer.clear();
            }
        }
    }
}

// ---------------------------------------------------------------- song builder task

fn song_builder_task(app: &mut App, ui: &mut Ui, verse_bars: u8, chorus_bars: u8) {
    let done =
        crate::screens::songs::SongsUi::has_song(app, verse_bars as usize, chorus_bars as usize);
    ui.label(
        RichText::new(format!(
            "Build a song with at least {verse_bars} bars in the verse and {chorus_bars} bars in the chorus, then save it."
        ))
        .size(18.0),
    );
    if done {
        ui.label(
            RichText::new("✔ You've saved a song that fits. Nice work, songwriter!")
                .size(18.0)
                .color(theme::GREEN),
        );
    }
    if ui
        .button(RichText::new("🎼 Open the Song Builder").size(18.0))
        .clicked()
    {
        app.screen = crate::app::Screen::Songs;
    }
    ui.label(
        RichText::new("Your lesson stays open. Click My Path to come back to it.")
            .small()
            .color(theme::SOFT_INK),
    );
}
