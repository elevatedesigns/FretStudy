//! Neck Explorer: any scale in any key across the neck, or one pentatonic box at a time.
//! Click any spot to hear it and learn its name and job.

use super::legend;
use crate::app::{App, NeckView};
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit::{self, card};
use crate::widgets::neck_map::NeckMap;
use crate::widgets::{ColorMode, Dot, DotShape, LabelMode};
use egui::{RichText, Ui};
use theory::chord::common_spelling;
use theory::shapes::pentatonic_box;
use theory::{NoteName, Role, Scale, ScaleKind};

const KINDS: [ScaleKind; 9] = [
    ScaleKind::MinorPentatonic,
    ScaleKind::MajorPentatonic,
    ScaleKind::Blues,
    ScaleKind::Major,
    ScaleKind::NaturalMinor,
    ScaleKind::Dorian,
    ScaleKind::Mixolydian,
    ScaleKind::HarmonicMinor,
    ScaleKind::Chromatic,
];

fn is_pentatonic(kind: ScaleKind) -> bool {
    matches!(
        kind,
        ScaleKind::MinorPentatonic | ScaleKind::MajorPentatonic | ScaleKind::Blues
    )
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        kit::page_header(
            ui,
            "Neck Explorer",
            "Explore the fretboard",
            "See scales, modes, and patterns across the whole neck. Click any note to hear it and learn its name.",
        );
        body(app, ui);
    });
}

fn body(app: &mut App, ui: &mut Ui) {
    let root = NoteName::from_pc(app.neck.root, common_spelling(app.neck.root));
    let scale = Scale::new(root, app.neck.kind);

    let pent = is_pentatonic(app.neck.kind) && app.neck.kind != ScaleKind::Blues;
    if !pent {
        app.neck.view = NeckView::WholeNeck;
    }
    // The neck always colors notes by their job; learners choose names or intervals.
    if app.style.label_mode == LabelMode::Finger {
        app.style.label_mode = LabelMode::Note;
    }
    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Key").color(theme::SOFT_INK));
                egui::ComboBox::from_id_salt("neck_root")
                    .selected_text(RichText::new(theme::glyphs(&root.pretty())).size(16.0))
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for pc in 0..12u8 {
                            let n = NoteName::from_pc(pc, common_spelling(pc));
                            ui.selectable_value(&mut app.neck.root, pc, theme::glyphs(&n.pretty()));
                        }
                    });
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("Scale").color(theme::SOFT_INK));
                egui::ComboBox::from_id_salt("neck_kind")
                    .selected_text(RichText::new(app.neck.kind.name()).size(16.0))
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for k in KINDS {
                            ui.selectable_value(&mut app.neck.kind, k, k.name());
                        }
                    });
            });
            if pent {
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Show").color(theme::SOFT_INK));
                    kit::segmented(
                        ui,
                        &mut app.neck.view,
                        &[
                            (NeckView::WholeNeck, "Whole neck"),
                            (NeckView::Box(1), "Shape 1"),
                            (NeckView::Box(2), "Shape 2"),
                            (NeckView::Box(3), "Shape 3"),
                            (NeckView::Box(4), "Shape 4"),
                            (NeckView::Box(5), "Shape 5"),
                        ],
                    );
                });
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Labels").color(theme::SOFT_INK));
            kit::segmented(
                ui,
                &mut app.style.label_mode,
                &[
                    (LabelMode::Note, "Note names"),
                    (LabelMode::Interval, "Intervals"),
                ],
            );
            ui.add_space(20.0);
            kit::toggle(ui, &mut app.style.left_handed, "Left-handed");
        });
        ui.separator();
        legend(ui, ColorMode::Role);
    });
    ui.add_space(14.0);

    // Which positions to show
    let tuning = app.tuning.clone();
    let box_notes: Option<Vec<(usize, u8)>> = match app.neck.view {
        NeckView::Box(n) => {
            // Pentatonic boxes are numbered from the relative minor (C major pentatonic = A minor boxes).
            let minor_root = if app.neck.kind == ScaleKind::MajorPentatonic {
                (app.neck.root + 9) % 12
            } else {
                app.neck.root
            };
            Some(
                pentatonic_box(minor_root, n, 0, &tuning)
                    .iter()
                    .map(|p| (p.string, p.fret))
                    .collect(),
            )
        }
        NeckView::WholeNeck => None,
    };
    let last_fret = 15u8;
    let mut dots = Vec::new();
    for s in 0..6 {
        for f in 0..=last_fret {
            let pc = tuning.pitch(s, f).pc();
            let Some(interval) = scale.interval_of(pc) else {
                continue;
            };
            let in_box = box_notes.as_ref().is_none_or(|b| b.contains(&(s, f)));
            let role = scale.role_of(pc).unwrap_or(Role::Other);
            let note = scale
                .notes()
                .into_iter()
                .find(|n| n.pc() == pc)
                .unwrap_or(root);
            let label = match app.style.label_mode {
                LabelMode::Interval => interval.label(),
                _ => note.pretty(),
            };
            dots.push(Dot {
                string: s,
                fret: f,
                fill: theme::role_color(role),
                label: theme::glyphs(&label),
                shape: match role {
                    Role::Root => DotShape::Square,
                    Role::BlueNote => DotShape::Diamond,
                    _ => DotShape::Circle,
                },
                faint: !in_box,
            });
        }
    }
    // Draw faint notes first so the box sits on top.
    dots.sort_by_key(|d| !d.faint);

    card(ui, |ui| {
        let title = format!("{} {}", root.pretty(), app.neck.kind.name());
        ui.horizontal(|ui| {
            ui.label(RichText::new(theme::glyphs(&title)).font(theme::heading(30.0)));
            let notes: Vec<String> = scale.notes().iter().map(|n| n.pretty()).collect();
            ui.label(
                RichText::new(theme::glyphs(&notes.join("  ")))
                    .color(theme::SOFT_INK)
                    .size(18.0),
            );
        });
        let highlight = app.sound.sequence_position().or(app.neck.last_clicked);
        let resp = NeckMap::new(&tuning, &dots)
            .frets(0, last_fret)
            .left_handed(app.style.left_handed)
            .height(280.0)
            .maple(true)
            .glow(app.sound.glow())
            .highlight(highlight)
            .show(ui);
        if let Some((s, f)) = resp.clicked {
            app.sound.note(&tuning, s, f);
            app.neck.last_clicked = Some((s, f));
        }

        ui.horizontal(|ui| {
            if let Some(box_notes) = &box_notes {
                if kit::primary(ui, Some(Icon::Play), "Play this shape up and down").clicked() {
                    let mut seq: Vec<(usize, u8)> = box_notes.clone();
                    let down: Vec<(usize, u8)> = seq.iter().rev().skip(1).copied().collect();
                    seq.extend(down);
                    app.sound.play_sequence(&tuning, &seq, 0.28);
                }
            } else if kit::primary(ui, Some(Icon::Play), "Play one octave").clicked() {
                // From the lowest root on string 6 (or 5), one octave up, using the nearest frets.
                let start_fret = tuning.fret_for(0, app.neck.root, 0);
                let start = tuning.pitch(0, start_fret).0;
                let mut seq = Vec::new();
                for i in scale
                    .kind
                    .intervals()
                    .iter()
                    .map(|i| i.semitones as i32)
                    .chain([12])
                {
                    let target = start + i;
                    let best = (0..6)
                        .filter_map(|s| {
                            let f = target - tuning.open[s].0;
                            (0..=last_fret as i32).contains(&f).then_some((s, f as u8))
                        })
                        .min_by_key(|(_, f)| (*f as i32 - start_fret as i32).abs());
                    if let Some(p) = best {
                        seq.push(p);
                    }
                }
                app.sound.play_sequence(&tuning, &seq, 0.32);
            }
            if kit::secondary(ui, Some(Icon::Stop), "Stop").clicked() {
                app.sound.stop();
            }
        });

        if let Some((s, f)) = app.neck.last_clicked {
            let pc = tuning.pitch(s, f).pc();
            let name = scale
                .notes()
                .into_iter()
                .find(|n| n.pc() == pc)
                .unwrap_or_else(|| NoteName::from_pc(pc, common_spelling(pc)));
            let text = match scale.interval_of(pc) {
                Some(i) => format!(
                    "That's {} (string {}, fret {f}). It's the {} of the scale ({}).",
                    name.pretty(),
                    theory::string_number(s),
                    i.label(),
                    i.name()
                ),
                None => format!(
                    "That's {} (string {}, fret {f}). It isn't in this scale.",
                    name.pretty(),
                    theory::string_number(s)
                ),
            };
            ui.label(RichText::new(theme::glyphs(&text)).size(17.0));
        } else {
            ui.label(
                RichText::new("Click any spot on the neck to hear it and learn its name.")
                    .color(theme::SOFT_INK),
            );
        }
    });
}
