//! The Song Builder: pick a key, drop chord numbers into sections (verse, chorus, bridge),
//! set the song's form, hear it with drums, record over it, and export a chord chart.

use crate::app::App;
use crate::lesson::visuals::schedule_groove;
use crate::recorder::RecorderUi;
use crate::theme;
use crate::widgets::icons::Icon;
use crate::widgets::kit;
use crate::widgets::theory_visuals;
use content::StrumPattern;
use egui::{Color32, CornerRadius, RichText, Sense, Ui, Vec2};
use serde::{Deserialize, Serialize};
use theory::{Chord, ChordQuality, Key, NoteName};

/// Keys offered in the key picker (common guitar spellings).
const KEYS: [&str; 12] = [
    "C", "G", "D", "A", "E", "B", "F#", "Db", "Ab", "Eb", "Bb", "F",
];

const PATTERNS: [StrumPattern; 8] = [
    StrumPattern::Quarters,
    StrumPattern::Eighths,
    StrumPattern::OldFaithful,
    StrumPattern::BoomChuck,
    StrumPattern::Swing8,
    StrumPattern::Sixteenths,
    StrumPattern::Pima,
    StrumPattern::Travis,
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub name: String,
    /// One chord per bar, as a number: "1"–"7", or a borrowed chord "b7" / "4m".
    pub bars: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub name: String,
    pub key: String,
    pub minor: bool,
    pub sevenths: bool,
    pub bpm: f32,
    pub pattern: StrumPattern,
    pub drums: bool,
    pub sections: Vec<Section>,
    /// The order sections are played in (indexes into `sections`).
    pub form: Vec<usize>,
}

impl Default for Song {
    fn default() -> Self {
        Song {
            name: "My Song".into(),
            key: "G".into(),
            minor: false,
            sevenths: false,
            bpm: 90.0,
            pattern: StrumPattern::OldFaithful,
            drums: true,
            sections: ["Verse", "Chorus", "Bridge"]
                .iter()
                .map(|n| Section {
                    name: n.to_string(),
                    bars: Vec::new(),
                })
                .collect(),
            form: vec![0, 1, 0, 1, 2, 1],
        }
    }
}

impl Song {
    fn key(&self) -> Option<Key> {
        let k = self.key.parse::<NoteName>().ok()?;
        Some(if self.minor {
            Key::minor(k)
        } else {
            Key::major(k)
        })
    }

    /// The chord a bar's number stands for in this song's key.
    pub fn chord(&self, number: &str) -> Option<Chord> {
        let key = self.key()?;
        let tonic = key.tonic;
        let borrowed = |semis: i32, quality| {
            let pc = (tonic.pc() as i32 + semis).rem_euclid(12) as theory::PitchClass;
            Chord::new(
                NoteName::from_pc(pc, theory::chord::common_spelling(pc)),
                quality,
            )
        };
        match number {
            "b7" => Some(borrowed(
                10,
                if self.sevenths {
                    ChordQuality::Dom7
                } else {
                    ChordQuality::Major
                },
            )),
            "4m" => Some(borrowed(
                5,
                if self.sevenths {
                    ChordQuality::Min7
                } else {
                    ChordQuality::Minor
                },
            )),
            n => {
                let i: usize = n.parse().ok()?;
                let chords = if self.sevenths {
                    key.sevenths()
                } else {
                    key.triads()
                };
                chords.get(i.checked_sub(1)?).map(|d| d.chord)
            }
        }
    }

    /// How a number is written on the chart.
    fn label(number: &str) -> String {
        match number {
            "b7" => "♭7".into(),
            "4m" => "4m".into(),
            n => n.to_string(),
        }
    }

    /// Every bar of the song in playing order: (section index, bar index).
    fn timeline(&self) -> Vec<(usize, usize)> {
        self.form
            .iter()
            .filter(|s| **s < self.sections.len())
            .flat_map(|s| (0..self.sections[*s].bars.len()).map(move |b| (*s, b)))
            .collect()
    }

    /// A plain-text chord chart.
    pub fn chart_text(&self) -> String {
        let mut out = format!(
            "{}\nKey: {}{} · {} BPM · {}\n\n",
            self.name,
            self.key,
            if self.minor { " minor" } else { " major" },
            self.bpm as u32,
            self.pattern.name()
        );
        for s in &self.sections {
            if s.bars.is_empty() {
                continue;
            }
            out.push_str(&format!("[{}]\n", s.name));
            for row in s.bars.chunks(4) {
                let cells: Vec<String> = row
                    .iter()
                    .map(|n| {
                        let name = self.chord(n).map(|c| c.pretty()).unwrap_or_default();
                        format!("{:<8}", format!("{name} ({})", Self::label(n)))
                    })
                    .collect();
                out.push_str(&format!("| {} |\n", cells.join(" | ")));
            }
            out.push('\n');
        }
        let form: Vec<&str> = self
            .form
            .iter()
            .filter_map(|i| self.sections.get(*i).map(|s| s.name.as_str()))
            .collect();
        out.push_str(&format!("Form: {}\n", form.join(" → ")));
        out
    }

    /// A printable HTML chord chart (open it in a browser and print, or save as PDF).
    pub fn chart_html(&self) -> String {
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        let mut body = String::new();
        for s in &self.sections {
            if s.bars.is_empty() {
                continue;
            }
            body.push_str(&format!("<h2>{}</h2><div class=\"bars\">", esc(&s.name)));
            for n in &s.bars {
                let name = self.chord(n).map(|c| c.pretty()).unwrap_or_default();
                body.push_str(&format!(
                    "<div class=\"bar\"><b>{}</b><span>{}</span></div>",
                    esc(&name),
                    esc(&Self::label(n))
                ));
            }
            body.push_str("</div>");
        }
        let form: Vec<String> = self
            .form
            .iter()
            .filter_map(|i| self.sections.get(*i).map(|s| esc(&s.name)))
            .collect();
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>{name}</title><style>\
             body{{font-family:-apple-system,Segoe UI,Arial,sans-serif;margin:40px;color:#2a2622}}\
             h1{{margin-bottom:4px}} .meta{{color:#6e665c;margin-bottom:24px}}\
             .bars{{display:grid;grid-template-columns:repeat(4,1fr);gap:8px}}\
             .bar{{border:2px solid #c9b28d;border-radius:10px;padding:14px;display:flex;justify-content:space-between;font-size:22px}}\
             .bar span{{color:#6e665c;font-size:16px}} .form{{margin-top:24px;font-size:18px}}\
             </style></head><body><h1>{name}</h1><div class=\"meta\">Key: {key}{mode} · {bpm} BPM · {pattern}</div>{body}\
             <div class=\"form\">Form: {form}</div><p class=\"meta\">Made with FretStudy</p></body></html>",
            name = esc(&self.name),
            key = esc(&self.key),
            mode = if self.minor { " minor" } else { " major" },
            bpm = self.bpm as u32,
            pattern = esc(self.pattern.name()),
            form = form.join(" → "),
        )
    }
}

#[derive(Default)]
pub struct SongsUi {
    pub song: Song,
    /// Database id of the song being edited (None = not saved yet).
    pub id: Option<i64>,
    /// Section that new chords go into.
    target: usize,
    /// Bar start times while the song plays, with their (section, bar).
    playing: Vec<(u64, (usize, usize))>,
    end_ns: u64,
    recorder: RecorderUi,
    message: Option<String>,
    saved: Option<Song>,
}

impl SongsUi {
    /// Does any saved song have at least these many bars in its verse and chorus?
    pub fn has_song(app: &App, verse: usize, chorus: usize) -> bool {
        let (Some(store), Some(p)) = (&app.store, &app.profile) else {
            return false;
        };
        store.songs(p.id).unwrap_or_default().iter().any(|s| {
            serde_json::from_str::<Song>(&s.data).is_ok_and(|song| {
                let bars = |name: &str| {
                    song.sections
                        .iter()
                        .filter(|s| s.name.eq_ignore_ascii_case(name))
                        .map(|s| s.bars.len())
                        .max()
                        .unwrap_or(0)
                };
                bars("verse") >= verse && bars("chorus") >= chorus
            })
        })
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let mut st = std::mem::take(&mut app.songs);
    egui::ScrollArea::vertical().show(ui, |ui| body(app, ui, &mut st));
    app.songs = st;
}

fn body(app: &mut App, ui: &mut Ui, st: &mut SongsUi) {
    kit::page_header(
        ui,
        "Songs",
        "Song Builder",
        "Pick a key, click chord numbers into your verse and chorus, and hear your song with drums.",
    );
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut st.song.name)
                .desired_width(220.0)
                .font(theme::heading(20.0)),
        );
        if kit::primary(ui, Some(Icon::Download), "Save").clicked() {
            save(app, st);
        }
        song_menu(app, ui, st);
        if kit::secondary(ui, Some(Icon::Upload), "Export chart").clicked() {
            export(st);
        }
    });
    if let Some(m) = &st.message {
        ui.label(RichText::new(m).color(theme::GREEN));
    }
    ui.add_space(6.0);
    // Key, feel, and speed.
    ui.horizontal_wrapped(|ui| {
        ui.label("Key");
        egui::ComboBox::from_id_salt("song_key")
            .selected_text(theme::glyphs(
                &st.song.key.replace('#', "♯").replace('b', "♭"),
            ))
            .show_ui(ui, |ui| {
                for k in KEYS {
                    let text = theme::glyphs(&k.replace('#', "♯").replace('b', "♭"));
                    ui.selectable_value(&mut st.song.key, k.to_string(), text);
                }
            });
        ui.selectable_value(&mut st.song.minor, false, "Major");
        ui.selectable_value(&mut st.song.minor, true, "Minor");
        ui.checkbox(&mut st.song.sevenths, "7th chords");
        ui.add_space(12.0);
        ui.label("Feel");
        egui::ComboBox::from_id_salt("song_pattern")
            .selected_text(st.song.pattern.name())
            .show_ui(ui, |ui| {
                for p in PATTERNS {
                    ui.selectable_value(&mut st.song.pattern, p, p.name());
                }
            });
        ui.add(
            egui::Slider::new(&mut st.song.bpm, 50.0..=160.0)
                .step_by(1.0)
                .suffix(" BPM"),
        );
        ui.checkbox(&mut st.song.drums, "Drums");
    });
    ui.add_space(8.0);

    // Chord palette: the key's family plus two favorite "guest" chords.
    ui.label(
        RichText::new(format!(
            "Click a chord to add it to the {}. Click a bar to take it out.",
            st.song
                .sections
                .get(st.target)
                .map(|s| s.name.to_lowercase())
                .unwrap_or_default()
        ))
        .color(theme::SOFT_INK),
    );
    if let Ok(k) = st.song.key.parse::<NoteName>() {
        let chords = theory_visuals::key_chords(k, st.song.sevenths, st.song.minor);
        if let Some(i) = theory_visuals::number_wheel(ui, &chords, ui.available_width().min(640.0))
        {
            add_bar(app, st, (i + 1).to_string());
        }
    }
    if !st.song.minor {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Guest chords:").color(theme::SOFT_INK));
            for n in ["b7", "4m"] {
                let name = st.song.chord(n).map(|c| c.pretty()).unwrap_or_default();
                if ui
                    .button(theme::glyphs(&format!("{} ({name})", Song::label(n))))
                    .clicked()
                {
                    add_bar(app, st, n.to_string());
                }
            }
        });
    }
    ui.add_space(8.0);

    // Sections.
    let now = app.sound.now_ns();
    let current = if now < st.end_ns {
        st.playing
            .iter()
            .rev()
            .find(|(t, _)| *t <= now)
            .map(|p| p.1)
    } else {
        None
    };
    let mut remove = None;
    for si in 0..st.song.sections.len() {
        let picked = st.target == si;
        ui.horizontal_wrapped(|ui| {
            let name = st.song.sections[si].name.clone();
            if ui
                .selectable_label(
                    picked,
                    RichText::new(format!("{name} ({})", st.song.sections[si].bars.len()))
                        .size(18.0),
                )
                .on_hover_text("New chords go here")
                .clicked()
            {
                st.target = si;
            }
            for (bi, n) in st.song.sections[si].bars.iter().enumerate() {
                let name = st.song.chord(n).map(|c| c.pretty()).unwrap_or_default();
                let lit = current == Some((si, bi));
                if bar_tile(ui, &theme::glyphs(&name), &Song::label(n), lit).clicked() {
                    remove = Some((si, bi));
                }
            }
            if ui
                .small_button("▶")
                .on_hover_text("Play this section")
                .clicked()
            {
                play(app, st, Some(si));
            }
        });
    }
    if let Some((s, b)) = remove {
        st.song.sections[s].bars.remove(b);
    }

    // Form.
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Form:").size(17.0));
        let mut drop = None;
        for (i, s) in st.song.form.iter().enumerate() {
            let name = st
                .song
                .sections
                .get(*s)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            if ui.button(name).on_hover_text("Click to take out").clicked() {
                drop = Some(i);
            }
        }
        if let Some(i) = drop {
            st.song.form.remove(i);
        }
        ui.label(RichText::new("Add:").color(theme::SOFT_INK));
        for i in 0..st.song.sections.len() {
            if ui
                .small_button(format!("+ {}", st.song.sections[i].name))
                .clicked()
            {
                st.song.form.push(i);
            }
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if kit::primary(ui, Some(Icon::Play), "Play the song").clicked() {
            play(app, st, None);
        }
        if kit::secondary(ui, Some(Icon::Stop), "Stop").clicked() {
            app.sound.cancel_scheduled();
            st.end_ns = 0;
        }
    });
    if current.is_some() {
        ui.ctx().request_repaint();
    }

    ui.add_space(12.0);
    ui.separator();
    ui.label(RichText::new("Record over your song").font(theme::heading(20.0)));
    ui.label(
        RichText::new(
            "Wear headphones, press Play, then Record. Your takes are saved on this computer.",
        )
        .color(theme::SOFT_INK),
    );
    let lesson = format!("song:{}", st.song.name);
    st.recorder.panel(app, ui, Some(&lesson), true, true);
}

fn bar_tile(ui: &mut Ui, name: &str, number: &str, lit: bool) -> egui::Response {
    let (r, painter) = ui.allocate_painter(Vec2::new(72.0, 46.0), Sense::click());
    let fill = if lit {
        theme::YELLOW
    } else if r.hovered() {
        theme::PANEL
    } else {
        Color32::WHITE
    };
    painter.rect(
        r.rect.shrink(2.0),
        CornerRadius::same(8),
        fill,
        egui::Stroke::new(1.5, theme::WOOD_DARK),
        egui::StrokeKind::Inside,
    );
    painter.text(
        r.rect.center() - Vec2::new(0.0, 7.0),
        egui::Align2::CENTER_CENTER,
        name,
        theme::heading(16.0),
        theme::INK,
    );
    painter.text(
        r.rect.center() + Vec2::new(0.0, 12.0),
        egui::Align2::CENTER_CENTER,
        theme::glyphs(number),
        egui::FontId::proportional(12.0),
        theme::SOFT_INK,
    );
    r.on_hover_text("Click to take this bar out")
}

fn add_bar(app: &mut App, st: &mut SongsUi, number: String) {
    if let Some(chord) = st.song.chord(&number)
        && let Some(shape) = theory::library::shape_for(&chord, &app.tuning)
    {
        app.sound
            .strum(&shape, &app.tuning, audio::play::StrumDirection::Down);
    }
    if let Some(s) = st.song.sections.get_mut(st.target) {
        s.bars.push(number);
    }
}

/// Plays the whole song (or one section) with the chosen feel, and drums if on.
fn play(app: &mut App, st: &mut SongsUi, section: Option<usize>) {
    let timeline: Vec<(usize, usize)> = match section {
        Some(s) => (0..st.song.sections[s].bars.len())
            .map(|b| (s, b))
            .collect(),
        None => st.song.timeline(),
    };
    let shapes: Vec<_> = timeline
        .iter()
        .filter_map(|(s, b)| {
            let chord = st.song.chord(&st.song.sections[*s].bars[*b])?;
            theory::library::shape_for(&chord, &app.tuning)
        })
        .collect();
    if shapes.is_empty() || shapes.len() != timeline.len() {
        st.message = Some("Add some chords first!".into());
        return;
    }
    app.sound.cancel_scheduled();
    if let Some((starts, end)) = schedule_groove(
        app,
        &shapes,
        st.song.pattern,
        st.song.bpm,
        !st.song.drums,
        st.song.drums,
    ) {
        st.playing = starts.into_iter().zip(timeline).collect();
        st.end_ns = end;
    }
}

fn save(app: &mut App, st: &mut SongsUi) {
    let (Some(store), Some(p)) = (&app.store, &app.profile) else {
        return;
    };
    let data = serde_json::to_string(&st.song).unwrap_or_default();
    match store.save_song(p.id, st.id, &st.song.name, &data) {
        Ok(id) => {
            st.id = Some(id);
            st.saved = Some(st.song.clone());
            st.message = Some(format!("Saved \"{}\".", st.song.name));
        }
        Err(e) => st.message = Some(format!("Couldn't save: {e:#}")),
    }
}

/// Open / new / delete.
fn song_menu(app: &mut App, ui: &mut Ui, st: &mut SongsUi) {
    let (Some(store), Some(p)) = (&app.store, &app.profile) else {
        return;
    };
    let songs = store.songs(p.id).unwrap_or_default();
    ui.menu_button("📂 My songs", |ui| {
        if ui.button("✚ New song").clicked() {
            st.song = Song::default();
            st.id = None;
            st.target = 0;
            st.message = None;
            ui.close();
        }
        if !songs.is_empty() {
            ui.separator();
        }
        for s in &songs {
            if ui.button(&s.name).clicked() {
                if let Ok(song) = serde_json::from_str::<Song>(&s.data) {
                    st.song = song;
                    st.id = Some(s.id);
                    st.target = 0;
                    st.message = None;
                }
                ui.close();
            }
        }
        if let Some(id) = st.id {
            ui.separator();
            if ui.button("🗑 Delete this song").clicked() {
                let _ = store.delete_song(id);
                st.song = Song::default();
                st.id = None;
                ui.close();
            }
        }
    });
}

/// Saves the chord chart as HTML (printable to PDF) or plain text.
fn export(st: &mut SongsUi) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{}.html", st.song.name))
        .add_filter("Printable chart (HTML)", &["html"])
        .add_filter("Plain text", &["txt"])
        .save_file()
    else {
        return;
    };
    let text = if path.extension().is_some_and(|e| e == "txt") {
        st.song.chart_text()
    } else {
        st.song.chart_html()
    };
    st.message = Some(match std::fs::write(&path, text) {
        Ok(()) => format!(
            "Chart saved to {}. Open it and print to make a PDF.",
            path.display()
        ),
        Err(e) => format!("Couldn't save the chart: {e}"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_become_chords_in_the_key() {
        let mut song = Song {
            key: "G".into(),
            ..Song::default()
        };
        let name = |s: &Song, n: &str| s.chord(n).unwrap().pretty();
        assert_eq!(name(&song, "1"), "G");
        assert_eq!(name(&song, "6"), "Em");
        assert_eq!(name(&song, "b7"), "F");
        assert_eq!(name(&song, "4m"), "Cm");
        song.sevenths = true;
        assert_eq!(name(&song, "5"), "D7");
        song.minor = true;
        song.key = "A".into();
        song.sevenths = false;
        assert_eq!(name(&song, "1"), "Am");
        assert_eq!(name(&song, "6"), "F");
    }

    #[test]
    fn every_key_chord_has_a_shape() {
        let t = theory::Tuning::standard();
        for key in KEYS {
            for minor in [false, true] {
                for sevenths in [false, true] {
                    let song = Song {
                        key: key.into(),
                        minor,
                        sevenths,
                        ..Song::default()
                    };
                    for n in ["1", "2", "3", "4", "5", "6", "7", "b7", "4m"] {
                        let chord = song.chord(n).unwrap();
                        let shape = theory::library::shape_for(&chord, &t).unwrap_or_else(|| {
                            panic!(
                                "no shape for {} in {key} (minor {minor}, 7ths {sevenths})",
                                chord.pretty()
                            )
                        });
                        assert!(
                            shape.check(&chord, &t).ok(),
                            "{shape} doesn't spell {}",
                            chord.pretty()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn chart_lists_sections_and_form() {
        let mut song = Song::default();
        song.sections[0].bars = vec!["1".into(), "4".into(), "5".into(), "1".into()];
        song.sections[1].bars = vec!["6".into(), "4".into()];
        let text = song.chart_text();
        assert!(text.contains("[Verse]"));
        assert!(text.contains("Em (6)"));
        assert!(text.contains("Form: Verse → Chorus → Verse → Chorus → Bridge → Chorus"));
        assert!(song.chart_html().contains("<h2>Chorus</h2>"));
    }
}
