//! Draws a lesson `Visual` (chord box, neck, tab, strum grid…), with sound where it helps.

use crate::app::App;
use crate::theme;
use crate::widgets::chord_box::ChordBox;
use crate::widgets::neck_map::NeckMap;
use crate::widgets::{
    ColorMode, DiagramStyle, Dot, DotShape, LabelMode, guitar_parts, hand, rhythm, tab_staff,
    theory_visuals,
};
use audio::play::StrumDirection;
use content::{StrumPattern, TabNote, Visual};
use egui::{RichText, Ui};
use theory::library;
use theory::{ChordShape, NoteName, Spelling, string_index};

/// Converts content tab notes (string 1–6) into (string index, fret, beats).
pub fn tab_positions(notes: &[TabNote]) -> Vec<(usize, u8, f32)> {
    notes
        .iter()
        .map(|n| (string_index(n.string.clamp(1, 6)), n.fret, n.beats))
        .collect()
}

/// Tab notes with their techniques, ready for `Sound::play_tab`.
pub fn tab_timed(notes: &[TabNote]) -> Vec<crate::sound::TimedNote> {
    notes
        .iter()
        .map(|n| (string_index(n.string.clamp(1, 6)), n.fret, n.beats, n.tech))
        .collect()
}

fn chord_card(app: &mut App, ui: &mut Ui, id: &str, width: f32) {
    let Some(entry) = library::find(id) else {
        ui.colored_label(theme::VERMILLION, format!("Unknown chord {id}"));
        return;
    };
    let shape = entry.shape();
    let chord = entry.chord();
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(entry.label).font(theme::heading(24.0)));
        });
        let resp = ChordBox::new(&shape, Some(&chord), &app.tuning)
            .style(app.style)
            .width(width)
            .glow(app.sound.glow())
            .show(ui);
        if let Some(s) = resp.clicked_string
            && let Some(f) = shape.frets[s]
        {
            app.sound.note(&app.tuning, s, f);
        }
        ui.horizontal(|ui| {
            if ui.button("⬇ Strum").clicked() {
                app.sound.strum(&shape, &app.tuning, StrumDirection::Down);
            }
            if ui.button("🔎 One at a time").clicked() {
                app.sound.arpeggio(&shape, &app.tuning);
            }
        });
    });
}

/// Draws a visual. `salt` keeps widget ids unique when several visuals share a screen.
pub fn show(app: &mut App, ui: &mut Ui, visual: &Visual, salt: &str) {
    ui.push_id(salt, |ui| match visual {
        Visual::GuitarParts => {
            let r = guitar_parts::GuitarParts::new(ui.available_width().min(760.0)).show(ui);
            if let Some(p) = r.hovered.or(r.clicked) {
                ui.label(RichText::new(format!("{}: {}", p.name(), p.about())).size(17.0));
            } else {
                ui.label(
                    RichText::new("Point at any part to learn its name.").color(theme::SOFT_INK),
                );
            }
        }
        Visual::Hand => {
            hand::hand(ui, 360.0);
        }
        Visual::Strings => {
            let r = hand::strings(ui, ui.available_width().min(640.0), None);
            if r.clicked() {
                // no-op; strings view is informational
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hear each open string:").color(theme::SOFT_INK));
                for n in (1..=6u8).rev() {
                    if ui.small_button(format!("{n}")).clicked() {
                        app.sound.note(&app.tuning, string_index(n), 0);
                    }
                }
            });
        }
        Visual::Chord(id) => chord_card(app, ui, id, 240.0),
        Visual::ChordPair(a, b) => {
            ui.horizontal_top(|ui| {
                chord_card(app, ui, a, 220.0);
                ui.add_space(24.0);
                chord_card(app, ui, b, 220.0);
            });
            if let (Some(x), Some(y)) = (library::find(a), library::find(b)) {
                let anchors = x.shape().anchor_fingers(&y.shape());
                if !anchors.is_empty() {
                    let list: Vec<String> = anchors.iter().map(|f| f.to_string()).collect();
                    ui.label(
                        RichText::new(format!(
                            "Anchor finger{}: {} stays put when you switch.",
                            if anchors.len() > 1 { "s" } else { "" },
                            list.join(" and ")
                        ))
                        .color(theme::GREEN),
                    );
                }
            }
        }
        Visual::Shape {
            frets,
            fingers,
            name,
        } => {
            if let Ok(shape) = ChordShape::parse(frets, Some(fingers)) {
                ui.vertical(|ui| {
                    if !name.is_empty() {
                        ui.label(RichText::new(name).font(theme::heading(22.0)));
                    }
                    // Fingers only: note names would give the answer away in quizzes.
                    let style = DiagramStyle {
                        color_mode: ColorMode::Finger,
                        label_mode: LabelMode::Finger,
                        ..app.style
                    };
                    let resp = ChordBox::new(&shape, None, &app.tuning)
                        .style(style)
                        .width(220.0)
                        .glow(app.sound.glow())
                        .show(ui);
                    if let Some(s) = resp.clicked_string
                        && let Some(f) = shape.frets[s]
                    {
                        app.sound.note(&app.tuning, s, f);
                    }
                    if ui.button("⬇ Strum").clicked() {
                        app.sound.strum(&shape, &app.tuning, StrumDirection::Down);
                    }
                });
            }
        }
        Visual::Neck {
            notes,
            first_fret,
            last_fret,
            show_names,
        } => {
            let dots: Vec<Dot> = notes
                .iter()
                .map(|n| {
                    let s = string_index(n.string.clamp(1, 6));
                    let name =
                        NoteName::from_pc(app.tuning.pitch(s, n.fret).pc(), Spelling::Sharps);
                    let finger = if n.fret == 0 {
                        None
                    } else {
                        Some(n.fret.min(4))
                    };
                    Dot {
                        string: s,
                        fret: n.fret,
                        fill: finger
                            .map(theme::finger_color)
                            .unwrap_or(egui::Color32::WHITE),
                        label: if *show_names {
                            theme::glyphs(&name.pretty())
                        } else {
                            String::new()
                        },
                        shape: DotShape::Circle,
                        faint: false,
                    }
                })
                .collect();
            let r = NeckMap::new(&app.tuning, &dots)
                .frets(*first_fret, *last_fret)
                .left_handed(app.style.left_handed)
                .height(210.0)
                .glow(app.sound.glow())
                .show(ui);
            if let Some((s, f)) = r.clicked {
                app.sound.note(&app.tuning, s, f);
            }
            ui.label(
                RichText::new("Click any spot to hear it.")
                    .small()
                    .color(theme::SOFT_INK),
            );
        }
        Visual::Tab(notes) => {
            let current = app.sound.sequence_index();
            let staff = app.style.show_staff.then_some((&app.tuning, None));
            tab_staff::notation_rows(ui, notes, current, 0, ui.available_width().min(700.0), 16, staff, 4);
            if ui.button("▶ Listen").clicked() {
                let pos = tab_timed(notes);
                app.sound.play_tab(&app.tuning.clone(), &pos, 80.0);
            }
        }
        Visual::StrumGrid(pattern) => {
            let playhead = app.sound.grid_slot(pattern.slots().len());
            rhythm::strum_grid(ui, *pattern, playhead, ui.available_width().min(560.0));
            let (chord, bpm) = match pattern {
                StrumPattern::Arpeggio68 => ("Am", 150.0),
                StrumPattern::BoomChuck | StrumPattern::Travis => ("C", 90.0),
                StrumPattern::Pima => ("Am", 80.0),
                StrumPattern::Boogie => ("A7", 90.0),
                _ => ("Em", 80.0),
            };
            let name = library::find(chord).map(|e| e.label).unwrap_or(chord);
            if ui.button(format!("▶ Hear the pattern on {name}")).clicked() {
                demo_pattern(app, *pattern, bpm, chord);
            }
        }
        Visual::Piano => {
            theory_visuals::piano(ui, ui.available_width().min(560.0));
        }
        Visual::Alphabet => {
            theory_visuals::alphabet(ui, ui.available_width().min(680.0));
        }
        Visual::Durations => {
            theory_visuals::durations(ui, ui.available_width().min(700.0));
        }
        Visual::ScaleSteps { root } => {
            if let Ok(root) = root.parse::<NoteName>() {
                theory_visuals::scale_steps(ui, root, ui.available_width().min(560.0));
                if ui.button("▶ Hear the scale").clicked() {
                    let start = 48 + root.pc() as i32;
                    let midis: Vec<i32> = theory::ScaleKind::Major
                        .intervals()
                        .iter()
                        .map(|i| start + i.semitones as i32)
                        .chain([start + 12])
                        .collect();
                    app.sound.play_pitches(&midis, 0.35);
                }
            }
        }
        Visual::NumberWheel {
            key,
            sevenths,
            minor,
        } => {
            let chords = key
                .parse::<NoteName>()
                .map(|k| theory_visuals::key_chords(k, *sevenths, *minor))
                .unwrap_or_default();
            if let Some(i) =
                theory_visuals::number_wheel(ui, &chords, ui.available_width().min(640.0))
            {
                let chord = chords[i].chord;
                let root = 48 + chord.root.pc() as i32;
                let midis: Vec<i32> = chord
                    .quality
                    .tones()
                    .iter()
                    .map(|t| root + t.interval.semitones as i32)
                    .collect();
                app.sound.play_pitches(&midis, 0.0);
            }
        }
        Visual::Capo { fret, chord } => {
            if let Some(entry) = library::find(chord) {
                let shape = entry.shape().shifted(*fret);
                let sounding = theory::shapes::capo_sounding(&entry.chord(), *fret);
                let style = DiagramStyle {
                    label_mode: LabelMode::Note,
                    ..app.style
                };
                let dots = crate::widgets::chord_dots(&shape, Some(&sounding), &app.tuning, &style);
                ui.label(
                    RichText::new(theme::glyphs(&format!(
                        "Capo on fret {fret} + {} shape = it sounds like {}",
                        entry.label,
                        sounding.pretty()
                    )))
                    .font(theme::heading(20.0)),
                );
                NeckMap::new(&app.tuning, &dots)
                    .frets(0, (*fret + 5).max(7))
                    .capo(Some(*fret))
                    .left_handed(app.style.left_handed)
                    .height(200.0)
                    .glow(app.sound.glow())
                    .show(ui);
                if ui
                    .button(format!(
                        "⬇ Strum it with the capo ({})",
                        theme::glyphs(&sounding.pretty())
                    ))
                    .clicked()
                {
                    app.sound.strum(&shape, &app.tuning, StrumDirection::Down);
                }
            }
        }
        Visual::ScaleBox {
            root,
            scale,
            shape,
            first_fret,
            last_fret,
        } => {
            if let (Ok(root), Some(kind)) = (root.parse::<NoteName>(), content::scale_kind(scale)) {
                let dots = theory_visuals::scale_dots(
                    &app.tuning,
                    root,
                    kind,
                    *shape,
                    (*first_fret, *last_fret),
                    app.style.label_mode,
                );
                ui.label(
                    RichText::new(theme::glyphs(&format!(
                        "{} {}{}",
                        root.pretty(),
                        kind.name(),
                        shape.map(|s| format!(" — box {s}")).unwrap_or_default()
                    )))
                    .font(theme::heading(20.0)),
                );
                let r = NeckMap::new(&app.tuning, &dots)
                    .frets(*first_fret, *last_fret)
                    .left_handed(app.style.left_handed)
                    .height(220.0)
                    .glow(app.sound.glow())
                    .show(ui);
                if let Some((s, f)) = r.clicked {
                    app.sound.note(&app.tuning, s, f);
                }
                if ui.button("▶ Hear the scale").clicked() {
                    let start = 45 + ((root.pc() + 12 - 9) % 12) as i32;
                    let mut midis: Vec<i32> = kind
                        .intervals()
                        .iter()
                        .map(|i| start + i.semitones as i32)
                        .collect();
                    midis.push(start + 12);
                    app.sound.play_pitches(&midis, 0.3);
                }
                ui.label(
                    RichText::new("Red squares = root · Click any dot to hear it.")
                        .small()
                        .color(theme::SOFT_INK),
                );
            }
        }
        Visual::Caged { root } => {
            if let Ok(root) = root.parse::<NoteName>() {
                caged_view(app, ui, root);
            }
        }
        Visual::TwelveBar { key } => {
            if let Ok(k) = key.parse::<NoteName>() {
                let playing = app
                    .sound
                    .grid_slot(usize::MAX)
                    .map(|i| i / StrumPattern::Boogie.slots().len());
                theory_visuals::twelve_bar(
                    ui,
                    k,
                    playing.filter(|b| *b < 12),
                    ui.available_width().min(560.0),
                );
                if ui.button("▶ Hear the 12 bars (shuffle)").clicked() {
                    let major = theory::Key::major(k);
                    let bars: Vec<ChordShape> = theory_visuals::TWELVE_BAR
                        .iter()
                        .filter_map(|n| {
                            let chord = theory::Chord::new(
                                major.chord_number(*n).root,
                                theory::ChordQuality::Dom7,
                            );
                            // Roots up to fret 7 on string 6, the rest on string 5.
                            theory::BarreFamily::EShape
                                .shape(&chord, &app.tuning)
                                .filter(|s| s.frets[0].is_some_and(|f| f <= 7))
                                .or_else(|| theory::BarreFamily::AShape.shape(&chord, &app.tuning))
                        })
                        .collect();
                    schedule_groove(app, &bars, StrumPattern::Boogie, 110.0, false, true);
                }
            }
        }
        Visual::ScaleCompare {
            root,
            a,
            b,
            first_fret,
            last_fret,
        } => {
            if let (Ok(root), Some(ka), Some(kb)) = (
                root.parse::<NoteName>(),
                content::scale_kind(a),
                content::scale_kind(b),
            ) {
                let dots = theory_visuals::compare_dots(
                    &app.tuning,
                    root,
                    ka,
                    kb,
                    (*first_fret, *last_fret),
                );
                ui.label(
                    RichText::new(theme::glyphs(&format!(
                        "{} {} + {}",
                        root.pretty(),
                        ka.name(),
                        kb.name()
                    )))
                    .font(theme::heading(20.0)),
                );
                let r = NeckMap::new(&app.tuning, &dots)
                    .frets(*first_fret, *last_fret)
                    .left_handed(app.style.left_handed)
                    .height(220.0)
                    .glow(app.sound.glow())
                    .show(ui);
                if let Some((s, f)) = r.clicked {
                    app.sound.note(&app.tuning, s, f);
                }
                ui.label(
                    RichText::new(theme::glyphs(&format!(
                        "Gray = in both · Sky blue circles = only in {} · Blue diamonds = only in {} · Red squares = root",
                        ka.name(),
                        kb.name()
                    )))
                    .small()
                    .color(theme::SOFT_INK),
                );
            }
        }
        Visual::Decoder { examples } => decoder(app, ui, examples),
        Visual::Staff { notes, key, bar } => {
            let key = key
                .as_ref()
                .and_then(|k| k.parse::<NoteName>().ok())
                .map(theory::Key::major);
            let current = app.sound.sequence_index();
            tab_staff::notation_rows(
                ui,
                notes,
                current,
                0,
                ui.available_width().min(700.0),
                12,
                Some((&app.tuning, key.as_ref())),
                *bar,
            );
            if ui.button("▶ Listen").clicked() {
                let pos = tab_timed(notes);
                app.sound.play_tab(&app.tuning.clone(), &pos, 70.0);
            }
        }
        Visual::BeatPulse { bpm } => {
            let (beat, flash) = app.sound.metronome_beat().unzip();
            rhythm::beat_pulse(ui, 4, beat, flash.unwrap_or(0.0), 360.0);
            ui.horizontal(|ui| {
                if ui
                    .button(format!("▶ Start metronome ({bpm:.0} BPM)"))
                    .clicked()
                {
                    app.sound.start_metronome(*bpm, 16);
                }
                if ui.button("■ Stop").clicked() {
                    app.sound.cancel_scheduled();
                }
            });
        }
    });
}

/// Plays two bars of a strum pattern on a chord, with the grid playhead following along.
pub fn demo_pattern(app: &mut App, pattern: StrumPattern, bpm: f32, chord_id: &str) {
    let Some(entry) = library::find(chord_id) else {
        return;
    };
    let shape = entry.shape();
    schedule_groove(app, &[shape.clone(), shape], pattern, bpm, true, false);
}

/// Schedules a backing groove: one chord shape per bar, played with `pattern` (swing
/// patterns land their off-beats late). Sets the strum-grid playhead and returns the time
/// (ns) each bar starts, plus when the groove ends.
pub fn schedule_groove(
    app: &mut App,
    bars: &[ChordShape],
    pattern: StrumPattern,
    bpm: f32,
    clicks: bool,
    drums: bool,
) -> Option<(Vec<u64>, u64)> {
    let start = app.sound.frame_after(0.15)?;
    let tuning = app.tuning.clone();
    let beat = app.sound.frames_for(60.0 / bpm as f64);
    let spb = pattern.slots_per_beat();
    let bpb = pattern.beats_per_bar();
    let slots = pattern.slots();
    let mut times = Vec::new();
    let mut bar_starts = Vec::new();
    for (bar, shape) in bars.iter().enumerate() {
        let bar_frame = start + (bar * bpb) as u64 * beat;
        bar_starts.push(app.sound.heard_ns(bar_frame));
        for b in 0..bpb {
            let beat_frame = bar_frame + b as u64 * beat;
            if clicks {
                app.sound.click(beat_frame, b == 0);
            }
            if drums {
                // Kick on 1 and 3, snare on 2 and 4, hi-hat on every slot.
                let drum = if b % 2 == 0 {
                    audio::Drum::Kick
                } else {
                    audio::Drum::Snare
                };
                app.sound.drum(beat_frame, drum);
                for k in 0..spb {
                    let f = beat_frame + (beat as f64 * pattern.slot_offset(k) as f64) as u64;
                    app.sound.drum(f, audio::Drum::HiHat);
                }
            }
            for k in 0..spb {
                let frame = beat_frame + (beat as f64 * pattern.slot_offset(k) as f64) as u64;
                times.push(app.sound.heard_ns(frame));
                app.sound
                    .schedule_stroke(shape, &tuning, frame, slots[b * spb + k], 0.75);
            }
        }
    }
    let end = app.sound.heard_ns(start + (bars.len() * bpb) as u64 * beat);
    app.sound.grid = times;
    Some((bar_starts, end))
}

/// One major chord in all five CAGED shapes; buttons pick which shape is lit.
fn caged_view(app: &mut App, ui: &mut Ui, root: NoteName) {
    let tuning = app.tuning.clone();
    let mut forms: Vec<(theory::Caged, ChordShape)> = theory::Caged::ORDER
        .iter()
        .map(|c| (*c, c.major_chord(root.pc(), &tuning)))
        .collect();
    let low = |s: &ChordShape| s.frets.iter().flatten().copied().min().unwrap_or(0);
    forms.sort_by_key(|(_, s)| low(s));
    let id = ui.id().with("caged_pick");
    let mut pick: usize = ui.data(|d| d.get_temp(id)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, (c, s)) in forms.iter().enumerate() {
            let hi = s.frets.iter().flatten().copied().max().unwrap_or(0);
            let text = format!("{c:?} shape (frets {}–{hi})", low(s));
            if ui
                .selectable_label(pick == i, RichText::new(text).size(16.0))
                .clicked()
            {
                pick = i;
                app.sound.strum(s, &tuning, StrumDirection::Down);
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(id, pick));
    let lit = &forms[pick.min(forms.len() - 1)].1;
    let mut dots = Vec::new();
    for s in 0..6 {
        for f in 0..=15u8 {
            let diff = (tuning.pitch(s, f).pc() + 12 - root.pc()) % 12;
            let (role, job) = match diff {
                0 => (theory::Role::Root, "R"),
                4 => (theory::Role::Third, "3"),
                7 => (theory::Role::Fifth, "5"),
                _ => continue,
            };
            let label = if app.style.label_mode == LabelMode::Note {
                theme::glyphs(
                    &NoteName::from_pc(
                        tuning.pitch(s, f).pc(),
                        theory::chord::common_spelling(root.pc()),
                    )
                    .pretty(),
                )
            } else {
                job.to_string()
            };
            dots.push(Dot {
                string: s,
                fret: f,
                fill: theme::role_color(role),
                label,
                shape: if role == theory::Role::Root {
                    DotShape::Square
                } else {
                    DotShape::Circle
                },
                faint: lit.frets[s] != Some(f),
            });
        }
    }
    NeckMap::new(&tuning, &dots)
        .frets(0, 15)
        .left_handed(app.style.left_handed)
        .height(220.0)
        .glow(app.sound.glow())
        .show(ui);
    ui.label(
        RichText::new("Red squares = root (R) · Blue = 3rd · Green = 5th. The same chord, five places on the neck.")
            .small()
            .color(theme::SOFT_INK),
    );
}

/// The chord-symbol decoder: type a symbol, see its recipe and notes, a shape, and hear it.
fn decoder(app: &mut App, ui: &mut Ui, examples: &[String]) {
    let id = ui.id().with("decoder_text");
    let mut text: String = ui
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| examples.first().cloned().unwrap_or_default());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Chord symbol:").size(18.0));
        ui.add(
            egui::TextEdit::singleline(&mut text)
                .desired_width(140.0)
                .font(theme::heading(20.0)),
        );
        for e in examples {
            if ui.small_button(theme::glyphs(e)).clicked() {
                text = e.clone();
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(id, text.clone()));
    let cleaned = text.replace('♯', "#").replace('♭', "b");
    let Ok(chord) = cleaned.parse::<theory::Chord>() else {
        ui.label(
            RichText::new(
                "Hmm, I can't read that one yet. Try a root letter and a kind, like Cm7 or F#m11.",
            )
            .color(theme::ORANGE),
        );
        return;
    };
    let recipe: Vec<String> = chord
        .quality
        .tones()
        .iter()
        .map(|t| t.interval.label().replace('R', "1"))
        .collect();
    let notes: Vec<String> = chord.notes().iter().map(|n| n.pretty()).collect();
    ui.label(
        RichText::new(theme::glyphs(&format!(
            "{} = {} chord",
            chord.pretty(),
            chord.quality.name()
        )))
        .font(theme::heading(22.0)),
    );
    ui.label(RichText::new(theme::glyphs(&format!("Recipe: {}", recipe.join(" – ")))).size(18.0));
    ui.label(RichText::new(theme::glyphs(&format!("Notes: {}", notes.join("  ")))).size(18.0));
    // A shape from the chord library that spells this chord, if there is one.
    let shape = library::CHORDS
        .iter()
        .find(|e| {
            e.chord().pitch_classes() == chord.pitch_classes()
                && e.chord().root.pc() == chord.root.pc()
        })
        .map(|e| e.shape())
        .or_else(|| {
            [theory::BarreFamily::AShape, theory::BarreFamily::EShape]
                .iter()
                .find_map(|f| f.shape(&chord, &app.tuning))
        });
    ui.horizontal_top(|ui| {
        if let Some(shape) = &shape {
            ChordBox::new(shape, Some(&chord), &app.tuning)
                .style(DiagramStyle {
                    color_mode: ColorMode::Role,
                    label_mode: LabelMode::Interval,
                    ..app.style
                })
                .width(200.0)
                .glow(app.sound.glow())
                .show(ui);
        } else {
            ui.label(
                RichText::new("No shape for this one yet. Simplify it: drop the color tones and play the plain chord.")
                    .color(theme::SOFT_INK),
            );
        }
        ui.vertical(|ui| {
            if ui.button("🔈 Hear it").clicked() {
                match &shape {
                    Some(s) => app.sound.strum(s, &app.tuning, StrumDirection::Down),
                    None => {
                        let root = 48 + chord.root.pc() as i32;
                        let midis: Vec<i32> = chord
                            .quality
                            .tones()
                            .iter()
                            .map(|t| root + t.interval.semitones as i32)
                            .collect();
                        app.sound.play_pitches(&midis, 0.0);
                    }
                }
            }
            if ui.button("🔎 One note at a time").clicked() {
                let root = 48 + chord.root.pc() as i32;
                let midis: Vec<i32> = chord
                    .quality
                    .tones()
                    .iter()
                    .map(|t| root + t.interval.semitones as i32)
                    .collect();
                app.sound.play_pitches(&midis, 0.4);
            }
        });
    });
}
