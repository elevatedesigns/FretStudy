//! Chord Explorer: a searchable chord library plus a finder for any chord, the chosen chord
//! as a big diagram you can hear, every way to play it (common shapes, inversions on each
//! string set, and full shapes up the neck), what's inside it, and where it sits on the neck.

use crate::app::App;
use crate::theme;
use crate::widgets::chord_box::{self, ChordBox, describe};
use crate::widgets::icons::{self, Icon};
use crate::widgets::kit::{self, card};
use crate::widgets::neck_map::NeckMap;
use crate::widgets::{ColorMode, DiagramStyle, LabelMode, chord_dots};
use audio::play::StrumDirection;
use egui::{Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use theory::library::{self, ChordEntry};
use theory::voicings::{self, Voicing};
use theory::{Chord, ChordQuality, ChordShape, NoteName, PitchClass};

/// Chord types offered by the finder, in a friendly order.
const FINDER_TYPES: [ChordQuality; 22] = [
    ChordQuality::Major,
    ChordQuality::Minor,
    ChordQuality::Dom7,
    ChordQuality::Maj7,
    ChordQuality::Min7,
    ChordQuality::Sus2,
    ChordQuality::Sus4,
    ChordQuality::Add9,
    ChordQuality::MinorAdd9,
    ChordQuality::Six,
    ChordQuality::MinorSix,
    ChordQuality::Dom9,
    ChordQuality::Maj9,
    ChordQuality::Min9,
    ChordQuality::Min11,
    ChordQuality::Dom13,
    ChordQuality::Dom7Sus4,
    ChordQuality::Dim,
    ChordQuality::Dim7,
    ChordQuality::HalfDim7,
    ChordQuality::Aug,
    ChordQuality::Power,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Ways {
    /// Shapes from the lessons' chord library.
    #[default]
    Common,
    /// Three- and four-note inversions on neighboring strings.
    Inversions,
    /// Full chords with the root in the bass, from the nut up.
    Neck,
}

#[derive(Default)]
pub struct ChordView {
    /// A chord picked in the finder (root, type) instead of a library shape.
    pub finder: Option<(PitchClass, ChordQuality)>,
    pub ways: Ways,
    /// Which shape in the current list is showing.
    pub pick: usize,
    cache: Option<(Chord, Vec<Voicing>, Vec<Voicing>)>,
}

/// The chord being explored and every way to play it.
struct Current {
    chord: Chord,
    title: String,
    entry: Option<&'static ChordEntry>,
    common: Vec<&'static ChordEntry>,
    inversions: Vec<Voicing>,
    neck: Vec<Voicing>,
}

impl Current {
    fn list_len(&self, ways: Ways) -> usize {
        match ways {
            Ways::Common => self.common.len(),
            Ways::Inversions => self.inversions.len(),
            Ways::Neck => self.neck.len(),
        }
    }

    /// The shape showing now and a line describing it.
    fn shape(&self, ways: Ways, pick: usize) -> Option<(ChordShape, String)> {
        match ways {
            Ways::Common => self
                .common
                .get(pick)
                .map(|e| (e.shape(), pretty_label(e.label))),
            Ways::Inversions => self
                .inversions
                .get(pick)
                .map(|v| (v.shape.clone(), v.label.clone())),
            Ways::Neck => self
                .neck
                .get(pick)
                .map(|v| (v.shape.clone(), v.label.clone())),
        }
    }
}

/// Library sections, in teaching order.
fn group_of(entry: &ChordEntry) -> &'static str {
    match entry.lesson.split('.').next().unwrap_or("") {
        "L0" | "L1" => "Basic Chords",
        "L2" => "Open Chords",
        "L3" => "7ths & Sus",
        "L4" => "Power & Barre Chords",
        "L5" => "Blues 7th Barres",
        "L6" => "Triads",
        "L7" => "7th & Color Chords",
        _ => "Style Tracks",
    }
}

fn pretty_label(label: &str) -> String {
    theme::glyphs(&label.replace('#', "♯").replace("Bb", "B♭"))
}

fn capitalized(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

fn note_name(pc: PitchClass) -> NoteName {
    NoteName::from_pc(pc, theory::chord::common_spelling(pc))
}

/// Library shapes that play exactly this chord (same root and notes).
fn common_shapes(chord: &Chord) -> Vec<&'static ChordEntry> {
    let pcs = chord.pitch_classes();
    library::CHORDS
        .iter()
        .filter(|e| {
            let c = e.chord();
            c.root.pc() == chord.root.pc() && c.pitch_classes() == pcs && c.bass.is_none()
        })
        .collect()
}

fn current(app: &mut App) -> Current {
    let (chord, title, entry) = match app.chord_view.finder {
        Some((pc, q)) => {
            let chord = Chord::new(note_name(pc), q);
            (chord, format!("{} {}", chord.root.pretty(), q.name()), None)
        }
        None => {
            let e = library::find(app.chord_id).unwrap_or(&library::CHORDS[0]);
            let c = e.chord();
            (
                c,
                format!("{} {}", c.root.pretty(), c.quality.name()),
                Some(e),
            )
        }
    };
    let cached = app.chord_view.cache.as_ref().is_some_and(|c| c.0 == chord);
    if !cached {
        app.chord_view.cache = Some((
            chord,
            voicings::inversions(&chord, &app.tuning),
            voicings::neck_shapes(&chord, &app.tuning),
        ));
    }
    let (inversions, neck) = match &app.chord_view.cache {
        Some((_, i, n)) => (i.clone(), n.clone()),
        None => (Vec::new(), Vec::new()),
    };
    Current {
        chord,
        title: theme::glyphs(&title),
        entry,
        common: common_shapes(&chord),
        inversions,
        neck,
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        kit::page_header(
            ui,
            "Chords",
            "Explore chords, build your skills.",
            "Learn guitar chords with interactive diagrams, sound, and visual guides. Find any chord, and every way to play it.",
        );
        let cur = current(app);
        // Keep the pick in range when the chord or list changes.
        let len = cur.list_len(app.chord_view.ways);
        if len == 0 && app.chord_view.ways == Ways::Common {
            app.chord_view.ways = if cur.neck.is_empty() { Ways::Inversions } else { Ways::Neck };
        }
        let len = cur.list_len(app.chord_view.ways);
        if app.chord_view.pick >= len {
            app.chord_view.pick = 0;
        }
        let full = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        let list_w = 280.0;
        let diagram_w = 330.0;
        let info_w = full - list_w - diagram_w - gap * 2.0 - 1.0;
        let top_h = 640.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(list_w, top_h), |ui| library_card(app, ui, list_w, top_h));
            ui.allocate_ui(Vec2::new(diagram_w, top_h), |ui| diagram_card(app, ui, &cur, diagram_w, top_h));
            ui.allocate_ui(Vec2::new(info_w, top_h), |ui| info_column(app, ui, &cur, info_w));
        });
        ui.add_space(14.0);
        ways_card(app, ui, &cur);
        ui.add_space(14.0);
        neck_card(app, ui, &cur);
    });
}

fn library_card(app: &mut App, ui: &mut Ui, width: f32, height: f32) {
    card(ui, |ui| {
        ui.set_width(width - 42.0);
        ui.set_height(height - 42.0);
        ui.label(RichText::new("Find any chord").font(theme::heading(19.0)));
        ui.horizontal(|ui| {
            let (mut root, mut q) = app.chord_view.finder.unwrap_or_else(|| {
                let c = library::find(app.chord_id).map(|e| e.chord());
                c.map(|c| (c.root.pc(), c.quality))
                    .unwrap_or((0, ChordQuality::Major))
            });
            let before = (root, q);
            egui::ComboBox::from_id_salt("finder_root")
                .selected_text(RichText::new(theme::glyphs(&note_name(root).pretty())).size(16.0))
                .width(64.0)
                .show_ui(ui, |ui| {
                    for pc in 0..12u8 {
                        ui.selectable_value(&mut root, pc, theme::glyphs(&note_name(pc).pretty()));
                    }
                });
            egui::ComboBox::from_id_salt("finder_type")
                .selected_text(RichText::new(capitalized(q.name())).size(15.0))
                .width(150.0)
                .height(360.0)
                .show_ui(ui, |ui| {
                    for t in FINDER_TYPES {
                        ui.selectable_value(&mut q, t, capitalized(t.name()));
                    }
                });
            if (root, q) != before {
                app.chord_view.finder = Some((root, q));
                app.chord_view.pick = 0;
                app.chord_view.ways = Ways::Common;
                let chord = Chord::new(note_name(root), q);
                if let Some(shape) = theory::library::shape_for(&chord, &app.tuning) {
                    app.sound.strum(&shape, &app.tuning, StrumDirection::Down);
                }
            }
        });
        ui.add_space(6.0);
        ui.separator();
        ui.label(RichText::new("Chord Library").font(theme::heading(19.0)));
        let search_id = ui.id().with("chord_search");
        let mut query: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::Search, 18.0, theme::SOFT_INK);
                    ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text("Search chords (e.g. C, G, Dm7)")
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        ui.data_mut(|d| d.insert_temp(search_id, query.clone()));
        ui.add_space(6.0);
        let favorites = app
            .profile
            .as_ref()
            .map(|p| p.settings.favorite_chords.clone())
            .unwrap_or_default();
        egui::ScrollArea::vertical().id_salt("chord_list").show(ui, |ui| {
            let q = query.trim().to_lowercase();
            if !q.is_empty() {
                let hits: Vec<&ChordEntry> = library::CHORDS
                    .iter()
                    .filter(|e| {
                        e.label.to_lowercase().contains(&q)
                            || e.symbol.to_lowercase().starts_with(&q)
                            || e.id.to_lowercase().starts_with(&q)
                    })
                    .collect();
                if hits.is_empty() {
                    ui.label(RichText::new("No chords match. Try a letter, like G, or use Find any chord above.").color(theme::SOFT_INK));
                }
                for e in hits {
                    chord_row(app, ui, e);
                }
                return;
            }
            let mut groups: Vec<(&str, Vec<&ChordEntry>)> = Vec::new();
            if !favorites.is_empty() {
                groups.push((
                    "★ Favorites",
                    favorites.iter().filter_map(|id| library::find(id)).collect(),
                ));
            }
            for e in library::CHORDS {
                let g = group_of(e);
                match groups.iter_mut().find(|(name, _)| *name == g) {
                    Some((_, list)) => list.push(e),
                    None => groups.push((g, vec![e])),
                }
            }
            for (name, entries) in groups {
                let open_id = ui.id().with(("group", name));
                let has_current = app.chord_view.finder.is_none() && entries.iter().any(|e| e.id == app.chord_id);
                let mut open: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(has_current || name == "Basic Chords");
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, name));
                if resp.clicked() {
                    open = !open;
                }
                let p = ui.painter();
                icons::paint(
                    p,
                    Rect::from_center_size(rect.left_center() + Vec2::new(10.0, 0.0), Vec2::splat(13.0)),
                    if open { Icon::ChevronDown } else { Icon::ChevronRight },
                    theme::INK,
                );
                p.text(rect.left_center() + Vec2::new(26.0, 0.0), egui::Align2::LEFT_CENTER, theme::glyphs(name), theme::heading(16.5), theme::INK);
                ui.data_mut(|d| d.insert_temp(open_id, open));
                if open {
                    for e in entries {
                        chord_row(app, ui, e);
                    }
                    ui.add_space(4.0);
                }
                ui.separator();
            }
        });
    });
}

/// One chord in the library list; the chosen one is orange.
fn chord_row(app: &mut App, ui: &mut Ui, e: &'static ChordEntry) {
    let selected = app.chord_view.finder.is_none() && app.chord_id == e.id;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 33.0), Sense::click());
    let label = pretty_label(e.label);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &label)
    });
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(8), theme::ACCENT_SOFT);
        p.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            CornerRadius::same(2),
            theme::ACCENT,
        );
    } else if resp.hovered() {
        p.rect_filled(
            rect,
            CornerRadius::same(8),
            Color32::from_rgb(0xFA, 0xF4, 0xEC),
        );
    }
    p.text(
        rect.left_center() + Vec2::new(26.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(16.0),
        if selected { theme::ACCENT } else { theme::INK },
    );
    if resp.clicked() {
        app.chord_id = e.id;
        app.chord_view.finder = None;
        app.chord_view.ways = Ways::Common;
        app.chord_view.pick = common_shapes(&e.chord())
            .iter()
            .position(|x| x.id == e.id)
            .unwrap_or(0);
        app.sound
            .strum(&e.shape(), &app.tuning, StrumDirection::Down);
    }
}

fn shape_kind(chord: &Chord, shape: &ChordShape) -> &'static str {
    if chord.quality == ChordQuality::Power {
        "Power chord"
    } else if shape.strings_played() <= 4 && !shape.frets.contains(&Some(0)) {
        "Movable shape"
    } else if !shape.barres.is_empty() {
        "Barre chord"
    } else if shape.frets.contains(&Some(0)) {
        "Open chord"
    } else {
        "Movable shape"
    }
}

fn diagram_card(app: &mut App, ui: &mut Ui, cur: &Current, width: f32, height: f32) {
    let ways = app.chord_view.ways;
    let pick = app.chord_view.pick;
    let Some((shape, label)) = cur.shape(ways, pick) else {
        card(ui, |ui| {
            ui.set_width(width - 42.0);
            ui.label(RichText::new(&cur.title).font(theme::heading(28.0)));
            ui.label(RichText::new("No playable shape found for this one.").color(theme::SOFT_INK));
        });
        return;
    };
    let chord = cur.chord;
    card(ui, |ui| {
        ui.set_width(width - 42.0);
        ui.set_height(height - 42.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(capitalized(&cur.title)).font(theme::heading(30.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if let Some(entry) = cur.entry {
                    let fav = app
                        .profile
                        .as_ref()
                        .is_some_and(|p| p.settings.favorite_chords.iter().any(|f| f == entry.id));
                    if star_button(ui, fav).clicked()
                        && let Some(p) = app.profile.as_mut()
                    {
                        let list = &mut p.settings.favorite_chords;
                        if fav {
                            list.retain(|f| f != entry.id);
                        } else {
                            list.push(entry.id.to_string());
                        }
                    }
                }
            });
        });
        ui.label(RichText::new(label).size(14.5).color(theme::SOFT_INK));
        ui.horizontal_wrapped(|ui| {
            kit::pill(
                ui,
                &chord.root.pretty(),
                theme::ACCENT_SOFT,
                theme::ACCENT_DARK,
            );
            kit::pill(
                ui,
                &capitalized(chord.quality.name()),
                Color32::from_rgb(0xF1, 0xEC, 0xE5),
                theme::INK,
            );
            kit::pill(
                ui,
                shape_kind(&chord, &shape),
                theme::GREEN_SOFT,
                Color32::from_rgb(0x0B, 0x6B, 0x4E),
            );
        });
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            let resp = ChordBox::new(&shape, Some(&chord), &app.tuning)
                .style(app.style)
                .width(width - 70.0)
                .glow(app.sound.glow())
                .show(ui);
            if let Some(s) = resp.clicked_string
                && let Some(f) = shape.frets[s]
            {
                app.sound.note(&app.tuning, s, f);
            }
            resp.response
                .on_hover_text("Click a string to hear just that string.");
        });
    });
}

fn star_button(ui: &mut Ui, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, "Favorite")
    });
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(10),
        Color32::WHITE,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    if on {
        let c = rect.center() + Vec2::new(0.0, 1.0);
        let pts: Vec<Pos2> = (0..10)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let r = if i % 2 == 0 { 12.0 } else { 5.2 };
                Pos2::new(c.x + a.cos() * r, c.y + a.sin() * r)
            })
            .collect();
        // Fan from the center so the concave star fills correctly.
        let mut mesh = egui::epaint::Mesh::default();
        mesh.colored_vertex(c, theme::ORANGE);
        for q in &pts {
            mesh.colored_vertex(*q, theme::ORANGE);
        }
        for i in 0..10u32 {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % 10);
        }
        p.add(egui::Shape::mesh(mesh));
    } else {
        icons::paint(p, rect.shrink(11.0), Icon::Star, theme::INK);
    }
    resp.on_hover_text(if on {
        "Remove from favorites"
    } else {
        "Add to favorites"
    })
}

fn info_column(app: &mut App, ui: &mut Ui, cur: &Current, width: f32) {
    let chord = cur.chord;
    let shape = cur
        .shape(app.chord_view.ways, app.chord_view.pick)
        .map(|s| s.0);
    let inner = width - 42.0;
    ui.vertical(|ui| {
        ui.set_width(width);
        card(ui, |ui| {
            ui.set_width(inner);
            ui.label(RichText::new("Hear it").font(theme::heading(22.0)));
            kit::subtitle(ui, "Play the chord to hear how it sounds.");
            ui.add_space(4.0);
            if let Some(shape) = &shape {
                ui.horizontal_wrapped(|ui| {
                    if kit::button(ui, kit::Kind::Soft, Some(Icon::ArrowDown), "Strum down").clicked() {
                        app.sound.strum(shape, &app.tuning, StrumDirection::Down);
                    }
                    if kit::secondary(ui, Some(Icon::ArrowUp), "Strum up").clicked() {
                        app.sound.strum(shape, &app.tuning, StrumDirection::Up);
                    }
                    if kit::secondary(ui, Some(Icon::Search), "One string at a time").clicked() {
                        app.sound.arpeggio(shape, &app.tuning);
                    }
                });
            }
            ui.add_space(4.0);
            kit::tip(ui, "Tip: click any string on the chord diagram to hear it alone. That's how the Buzz Detective finds a muted string.");
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.set_width(inner);
            kit::title(ui, Some(Icon::Book), "What's inside");
            let notes: Vec<String> = chord
                .notes()
                .iter()
                .zip(chord.quality.tones())
                .map(|(n, t)| format!("{} ({})", n.pretty(), t.interval.label()))
                .collect();
            ui.label(
                RichText::new(theme::glyphs(&format!(
                    "{} is a {} chord: {}",
                    chord.pretty(),
                    chord.quality.name(),
                    notes.join(", ")
                )))
                .size(16.5)
                .strong(),
            );
            match cur.entry {
                Some(e) if app.chord_view.ways == Ways::Common => {
                    ui.label(RichText::new(format!("Introduced in lesson {}.", e.lesson)).color(theme::SOFT_INK));
                }
                _ => {}
            }
            if let Some(shape) = &shape {
                kit::inset(ui, Color32::from_rgb(0xF6, 0xF2, 0xEC), |ui| {
                    ui.label(RichText::new(describe(shape, &cur.title)).size(14.5).color(theme::SOFT_INK));
                });
            }
        });
    });
}

/// Every way to play the chord, as small diagrams to click through.
fn ways_card(app: &mut App, ui: &mut Ui, cur: &Current) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ways to play it").font(theme::heading(22.0)));
            ui.add_space(12.0);
            let common = format!("Common shapes ({})", cur.common.len());
            let inv = format!("Inversions ({})", cur.inversions.len());
            let neck = format!("Up the neck ({})", cur.neck.len());
            let mut ways = app.chord_view.ways;
            if kit::segmented(
                ui,
                &mut ways,
                &[
                    (Ways::Common, common.as_str()),
                    (Ways::Inversions, inv.as_str()),
                    (Ways::Neck, neck.as_str()),
                ],
            ) {
                app.chord_view.ways = ways;
                app.chord_view.pick = 0;
            }
        });
        let help = match app.chord_view.ways {
            Ways::Common => "The shapes taught in the lessons.",
            Ways::Inversions => {
                "The chord's notes stacked in a new order on three or four neighboring strings. Root position has the root on the bottom; 1st inversion has the 3rd on the bottom; 2nd inversion has the 5th on the bottom."
            }
            Ways::Neck => {
                "Full chords with the root in the bass, from the nut up the neck, like the five CAGED positions."
            }
        };
        kit::subtitle(ui, help);
        ui.add_space(6.0);
        let items: Vec<(ChordShape, String)> = (0..cur.list_len(app.chord_view.ways))
            .filter_map(|i| cur.shape(app.chord_view.ways, i))
            .collect();
        if items.is_empty() {
            ui.label(
                RichText::new("None for this chord yet. Try another tab.").color(theme::SOFT_INK),
            );
            return;
        }
        let tile_w = 132.0;
        // Lay tiles out in rows that fit the card (frames don't wrap on their own).
        let per_row = ((ui.available_width() + 10.0) / (tile_w + 16.0 + 10.0))
            .floor()
            .max(1.0) as usize;
        for (r, row) in items.chunks(per_row).enumerate() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                for (k, (shape, label)) in row.iter().enumerate() {
                    let i = r * per_row + k;
                    let selected = app.chord_view.pick == i;
                    let fill = if selected {
                        theme::ACCENT_SOFT
                    } else {
                        Color32::WHITE
                    };
                    let rect = egui::Frame::new()
                        .fill(fill)
                        .stroke(Stroke::new(
                            if selected { 2.0 } else { 1.0 },
                            if selected {
                                theme::ACCENT
                            } else {
                                theme::BORDER
                            },
                        ))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(tile_w);
                            ui.vertical_centered(|ui| {
                                chord_box::mini(
                                    ui,
                                    shape,
                                    &theme::glyphs(&cur.chord.pretty()),
                                    tile_w - 14.0,
                                );
                                let short = label
                                    .replace("Strings ", "")
                                    .replace("Root position", "root pos.")
                                    .replace(" inversion", " inv.")
                                    .replace(" · root on string", " · string")
                                    .replace("Open position", "Open");
                                ui.label(RichText::new(short).size(12.5).color(theme::SOFT_INK));
                            });
                        })
                        .response
                        .rect;
                    let resp = ui.interact(rect, ui.id().with(("way", i)), Sense::click());
                    resp.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            selected,
                            label,
                        )
                    });
                    if resp.clicked() {
                        app.chord_view.pick = i;
                        app.sound.strum(shape, &app.tuning, StrumDirection::Down);
                    }
                }
            });
            ui.add_space(4.0);
        }
    });
}

fn neck_card(app: &mut App, ui: &mut Ui, cur: &Current) {
    let Some((shape, _)) = cur.shape(app.chord_view.ways, app.chord_view.pick) else {
        return;
    };
    let chord = cur.chord;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            ui.painter()
                .circle_filled(r.center(), 22.0, theme::ACCENT_SOFT);
            icons::paint(ui.painter(), r.shrink(11.0), Icon::Guitar, theme::ACCENT);
            ui.vertical(|ui| {
                ui.label(RichText::new("On the whole neck").font(theme::heading(22.0)));
                kit::subtitle(ui, "See where this shape's notes sit on the fretboard.");
            });
        });
        // The neck map can't show finger numbers for notes outside the shape, so it uses
        // note jobs or names unless the learner chose finger mode.
        let neck_style = DiagramStyle {
            label_mode: if app.style.label_mode == LabelMode::Finger
                && app.style.color_mode == ColorMode::Role
            {
                LabelMode::Interval
            } else {
                app.style.label_mode
            },
            ..app.style
        };
        let dots = chord_dots(&shape, Some(&chord), &app.tuning, &neck_style);
        let last = shape.highest_fret().unwrap_or(0).max(12);
        let resp = NeckMap::new(&app.tuning, &dots)
            .frets(0, last)
            .left_handed(app.style.left_handed)
            .height(200.0)
            .glow(app.sound.glow())
            .show(ui);
        if let Some((s, f)) = resp.clicked {
            app.sound.note(&app.tuning, s, f);
        }
    });
}
