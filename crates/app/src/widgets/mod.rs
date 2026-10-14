//! Diagram widgets. Every diagram is drawn from data (chord shapes, scale notes),
//! never from images, so one renderer keeps the whole curriculum consistent.

pub mod chord_box;
pub mod describe;
pub mod guitar_parts;
pub mod hand;
pub mod icons;
pub mod kit;
pub mod neck_map;
pub mod needle;
pub mod rhythm;
pub mod staff;
pub mod tab_staff;
pub mod theory_visuals;

use crate::theme;
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use theory::{Chord, ChordShape, NoteName, Spelling, Tuning};

/// What the dot colors mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// Which fretting finger (blue 1, green 2, orange 3, purple 4).
    Finger,
    /// The note's job: root, 3rd, 5th, 7th, other.
    Role,
}

/// What text goes inside each dot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelMode {
    Finger,
    Note,
    Interval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DotShape {
    Circle,
    /// Used for roots so they don't rely on color alone.
    Square,
    /// Used for the blues ♭5.
    Diamond,
}

/// One marked position on a diagram.
#[derive(Clone, Debug, PartialEq)]
pub struct Dot {
    pub string: usize,
    pub fret: u8,
    pub fill: Color32,
    pub label: String,
    pub shape: DotShape,
    /// Faded (for example, other notes of a scale outside the current box).
    pub faint: bool,
}

/// Shared display options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagramStyle {
    pub color_mode: ColorMode,
    pub label_mode: LabelMode,
    pub left_handed: bool,
    /// Draw standard notation above tab.
    pub show_staff: bool,
}

impl Default for DiagramStyle {
    fn default() -> Self {
        DiagramStyle {
            color_mode: ColorMode::Finger,
            label_mode: LabelMode::Finger,
            left_handed: false,
            show_staff: false,
        }
    }
}

/// Builds the dots for a chord shape according to the display style.
pub fn chord_dots(
    shape: &ChordShape,
    chord: Option<&Chord>,
    tuning: &Tuning,
    style: &DiagramStyle,
) -> Vec<Dot> {
    let spelling = chord
        .map(|c| spelling_for(c.root))
        .unwrap_or(Spelling::Sharps);
    (0..shape.frets.len())
        .filter_map(|s| {
            let fret = shape.frets[s]?;
            let pc = tuning.pitch(s, fret).pc();
            let interval = chord.and_then(|c| c.interval_of(pc));
            let role = interval.map(|i| i.role());
            let finger = shape.fingers[s];
            let fill = match style.color_mode {
                // Open strings are hollow, matching the "O" above a chord box.
                ColorMode::Finger if fret == 0 => Color32::WHITE,
                ColorMode::Finger => finger.map(theme::finger_color).unwrap_or(theme::GRAY),
                ColorMode::Role => role.map(theme::role_color).unwrap_or(theme::GRAY),
            };
            let note_name = chord
                .and_then(|c| c.notes().into_iter().find(|n| n.pc() == pc))
                .unwrap_or_else(|| NoteName::from_pc(pc, spelling));
            let label = match style.label_mode {
                LabelMode::Finger if fret == 0 => "O".into(),
                LabelMode::Finger => finger
                    .map(|f| if f == 5 { "T".into() } else { f.to_string() })
                    .unwrap_or_default(),
                LabelMode::Note => note_name.pretty(),
                LabelMode::Interval => interval.map(|i| i.label()).unwrap_or_default(),
            };
            let shape_kind =
                if style.color_mode == ColorMode::Role && role == Some(theory::Role::Root) {
                    DotShape::Square
                } else {
                    DotShape::Circle
                };
            Some(Dot {
                string: s,
                fret,
                fill,
                label: theme::glyphs(&label),
                shape: shape_kind,
                faint: false,
            })
        })
        .collect()
}

/// Flats for flat keys, sharps otherwise.
pub fn spelling_for(root: NoteName) -> Spelling {
    if root.accidental < 0 || root == NoteName::natural(theory::Letter::F) {
        Spelling::Flats
    } else {
        Spelling::Sharps
    }
}

/// Paints a dot with its label, sized to `radius`.
pub fn paint_dot(painter: &Painter, center: Pos2, radius: f32, dot: &Dot, glow: f32) {
    let alpha = if dot.faint { 0.35 } else { 1.0 };
    let fill = dot.fill.gamma_multiply(alpha);
    if glow > 0.0 {
        painter.circle_filled(
            center,
            radius * (1.0 + 0.5 * glow),
            dot.fill.gamma_multiply(0.35 * glow),
        );
    }
    let outline = Stroke::new(1.5, theme::INK.gamma_multiply(0.55 * alpha));
    match dot.shape {
        DotShape::Circle => {
            painter.circle_filled(center, radius, fill);
            painter.circle_stroke(center, radius, outline);
        }
        DotShape::Square => {
            let r = Rect::from_center_size(center, Vec2::splat(radius * 1.9));
            painter.rect_filled(r, CornerRadius::same((radius * 0.35) as u8), fill);
            painter.rect_stroke(
                r,
                CornerRadius::same((radius * 0.35) as u8),
                outline,
                egui::StrokeKind::Middle,
            );
        }
        DotShape::Diamond => {
            let d = radius * 1.25;
            let pts = vec![
                center + Vec2::new(0.0, -d),
                center + Vec2::new(d, 0.0),
                center + Vec2::new(0.0, d),
                center + Vec2::new(-d, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(pts, fill, outline));
        }
    }
    if !dot.label.is_empty() {
        let size = if dot.label.chars().count() >= 3 {
            radius * 0.85
        } else {
            radius * 1.1
        };
        let text = theme::text_on(dot.fill).gamma_multiply(alpha);
        painter.text(
            center,
            Align2::CENTER_CENTER,
            &dot.label,
            FontId::proportional(size),
            text,
        );
    }
}
