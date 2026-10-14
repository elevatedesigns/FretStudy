//! The app's building blocks: cards, buttons, segmented controls, toggles, pills, progress
//! rings, tips, and the page header with its painted landscape. Every screen uses these so
//! the whole app shares one look.

use super::icons::{self, Icon};
use crate::theme;
use egui::epaint::{Mesh, PathShape, PathStroke, Vertex};
use egui::{
    Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType,
};

// ---------------------------------------------------------------- cards

/// A white card with a hairline border and a soft shadow.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    card_tinted(ui, Color32::WHITE, add)
}

/// A card with its own fill color.
pub fn card_tinted<R>(ui: &mut Ui, fill: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // In a column, cards fill the width like blocks on a page. In a row, they hug
    // their content so several fit side by side.
    let fill_width = !ui.layout().main_dir().is_horizontal();
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(16))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .shadow(egui::Shadow {
            offset: [0, 2],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(10),
        })
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            if fill_width {
                ui.set_min_width(ui.available_width());
            }
            ui.vertical(add).inner
        })
        .inner
}

/// A lightly tinted inner panel (inside cards).
pub fn inset<R>(ui: &mut Ui, fill: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| ui.vertical(add).inner)
        .inner
}

/// A bold card title, with an optional icon.
pub fn title(ui: &mut Ui, icon: Option<Icon>, text: &str) {
    ui.horizontal(|ui| {
        if let Some(i) = icon {
            icons::show(ui, i, 24.0, theme::INK);
        }
        ui.label(RichText::new(text).font(theme::heading(22.0)));
    });
}

/// Smaller grey text under a title.
pub fn subtitle(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).color(theme::SOFT_INK));
}

// ---------------------------------------------------------------- buttons

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Orange, white text: the main action on a screen.
    Primary,
    /// White with a border.
    Secondary,
    /// Pale red: destructive actions.
    Danger,
    /// Pale orange: a chosen option shown as a button.
    Soft,
}

/// A button with an optional icon. Screen readers hear `text`.
pub fn button(ui: &mut Ui, kind: Kind, icon: Option<Icon>, text: &str) -> Response {
    button_sized(ui, kind, icon, text, 16.0)
}

/// Main call to action.
pub fn primary(ui: &mut Ui, icon: Option<Icon>, text: &str) -> Response {
    button_sized(ui, Kind::Primary, icon, text, 17.0)
}

/// A quieter action.
pub fn secondary(ui: &mut Ui, icon: Option<Icon>, text: &str) -> Response {
    button(ui, Kind::Secondary, icon, text)
}

pub fn button_sized(
    ui: &mut Ui,
    kind: Kind,
    icon: Option<Icon>,
    text: &str,
    size: f32,
) -> Response {
    let enabled = ui.is_enabled();
    let font = if kind == Kind::Primary {
        theme::heading(size)
    } else {
        FontId::proportional(size)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(theme::glyphs(text), font, Color32::PLACEHOLDER);
    let icon_w = if icon.is_some() { size + 8.0 } else { 0.0 };
    let pad = if kind == Kind::Primary { 22.0 } else { 16.0 };
    let h = if kind == Kind::Primary {
        size * 2.6
    } else {
        size * 2.4
    };
    let size_v = Vec2::new(galley.size().x + icon_w + pad * 2.0, h);
    let (rect, resp) = ui.allocate_exact_size(size_v, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, text));
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered() && enabled;
        let down = resp.is_pointer_button_down_on() && enabled;
        let (fill, stroke, ink) = match kind {
            Kind::Primary => (
                if down {
                    theme::ACCENT_DARK
                } else if hovered {
                    Color32::from_rgb(0xF0, 0x64, 0x2C)
                } else {
                    theme::ACCENT
                },
                Stroke::NONE,
                Color32::WHITE,
            ),
            Kind::Secondary => (
                if hovered {
                    Color32::from_rgb(0xFF, 0xF4, 0xEC)
                } else {
                    Color32::WHITE
                },
                Stroke::new(
                    1.0,
                    if hovered {
                        Color32::from_rgb(0xF2, 0xB8, 0x95)
                    } else {
                        theme::BORDER
                    },
                ),
                theme::INK,
            ),
            Kind::Danger => (
                if hovered {
                    Color32::from_rgb(0xFC, 0xDD, 0xD6)
                } else {
                    Color32::from_rgb(0xFD, 0xEC, 0xE8)
                },
                Stroke::NONE,
                Color32::from_rgb(0xC6, 0x2B, 0x10),
            ),
            Kind::Soft => (
                theme::ACCENT_SOFT,
                Stroke::new(1.0, Color32::from_rgb(0xF6, 0xC9, 0xAE)),
                theme::ACCENT_DARK,
            ),
        };
        let (fill, ink) = if enabled {
            (fill, ink)
        } else {
            (
                Color32::from_rgb(0xEE, 0xEA, 0xE4),
                Color32::from_rgb(0xA8, 0xA2, 0x9A),
            )
        };
        let p = ui.painter();
        p.rect(
            rect,
            CornerRadius::same(10),
            fill,
            stroke,
            StrokeKind::Inside,
        );
        let mut x = rect.left() + pad;
        if let Some(i) = icon {
            icons::paint(
                p,
                Rect::from_center_size(
                    Pos2::new(x + size / 2.0, rect.center().y),
                    Vec2::splat(size),
                ),
                i,
                ink,
            );
            x += icon_w;
        }
        p.galley(
            Pos2::new(x, rect.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
    }
    resp
}

/// A small square icon-only button (e.g. refresh). `label` is read by screen readers.
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    let fill = if resp.hovered() {
        Color32::from_rgb(0xFF, 0xF4, 0xEC)
    } else {
        Color32::WHITE
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        fill,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
    icons::paint(ui.painter(), rect.shrink(11.0), icon, theme::INK);
    resp.on_hover_text(label)
}

// ---------------------------------------------------------------- choices

/// A row of joined options; the chosen one is orange. Returns true when it changed.
pub fn segmented<T: PartialEq + Clone>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    let font = FontId::proportional(15.5);
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, l)| {
            ui.painter()
                .layout_no_wrap(theme::glyphs(l), font.clone(), Color32::PLACEHOLDER)
        })
        .collect();
    let h = 40.0;
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 32.0).collect();
    let total: f32 = widths.iter().sum();
    let (outer, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());
    ui.painter().rect(
        outer,
        CornerRadius::same(10),
        Color32::WHITE,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
    let mut x = outer.left();
    for (i, ((value, label), galley)) in options.iter().zip(galleys).enumerate() {
        let r = Rect::from_min_size(Pos2::new(x, outer.top()), Vec2::new(widths[i], h));
        x += widths[i];
        let id = ui.id().with(("seg", label));
        let resp = ui.interact(r, id, Sense::click());
        let selected = *current == *value;
        resp.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, *label)
        });
        if resp.clicked() && !selected {
            *current = value.clone();
            changed = true;
        }
        let p = ui.painter();
        if selected {
            p.rect_filled(r.shrink(3.0), CornerRadius::same(8), theme::ACCENT);
        } else if resp.hovered() {
            p.rect_filled(
                r.shrink(3.0),
                CornerRadius::same(8),
                Color32::from_rgb(0xFF, 0xF4, 0xEC),
            );
        }
        if i > 0 && !selected {
            p.line_segment(
                [
                    Pos2::new(r.left(), r.top() + 9.0),
                    Pos2::new(r.left(), r.bottom() - 9.0),
                ],
                Stroke::new(1.0, theme::BORDER),
            );
        }
        let ink = if selected { Color32::WHITE } else { theme::INK };
        p.galley(r.center() - galley.size() / 2.0, galley, ink);
    }
    changed
}

/// An on/off switch with a label to its left.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(15.5));
        let size = Vec2::new(50.0, 28.0);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
        if resp.clicked() {
            *on = !*on;
            resp.mark_changed();
        }
        resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, *on, label));
        let t = ui.ctx().animate_bool_responsive(resp.id, *on);
        let off = Color32::from_rgb(0xDD, 0xD6, 0xCC);
        let fill = off.lerp_to_gamma(theme::ACCENT, t);
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(14), fill);
        let x = egui::lerp(rect.left() + 14.0..=rect.right() - 14.0, t);
        p.circle(
            Pos2::new(x, rect.center().y),
            11.0,
            Color32::WHITE,
            Stroke::new(1.0, Color32::from_black_alpha(25)),
        );
        resp
    })
    .inner
}

/// A small rounded tag.
pub fn pill(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(theme::glyphs(text), FontId::proportional(14.0), ink);
    let (rect, resp) =
        ui.allocate_exact_size(galley.size() + Vec2::new(26.0, 12.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(255), fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ink);
    resp
}

// ---------------------------------------------------------------- progress

/// A horizontal progress bar.
pub fn progress_bar(ui: &mut Ui, fraction: f32, color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 12.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(6),
        Color32::from_rgb(0xEF, 0xEA, 0xE2),
    );
    let mut fill = rect;
    fill.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    if fill.width() > 0.5 {
        p.rect_filled(fill, CornerRadius::same(6), color);
    }
}

fn arc_points(c: Pos2, r: f32, from: f32, to: f32) -> Vec<Pos2> {
    let n = ((to - from).abs() * r / 3.0).ceil().max(2.0) as usize;
    (0..=n)
        .map(|i| {
            let a = from + (to - from) * i as f32 / n as f32;
            Pos2::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// A ring that fills clockwise from the top, with big and small text in the middle.
pub fn progress_ring(ui: &mut Ui, fraction: f32, size: f32, big: &str, small: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let r = size / 2.0 - 8.0;
    p.circle_stroke(c, r, Stroke::new(12.0, Color32::from_rgb(0xF1, 0xEC, 0xE4)));
    let start = -std::f32::consts::FRAC_PI_2;
    let end = start + std::f32::consts::TAU * fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        p.add(Shape::Path(PathShape::line(
            arc_points(c, r, start, end),
            PathStroke::new(12.0, theme::SKY_BLUE),
        )));
    }
    p.text(
        c - Vec2::new(0.0, size * 0.07),
        Align2::CENTER_CENTER,
        big,
        theme::heading(size * 0.24),
        theme::INK,
    );
    // The caption must fit inside the ring's hole, so it shrinks until it does.
    let hole = 2.0 * (r - 8.0);
    let mut font = 13.0;
    while font > 9.0
        && p.layout_no_wrap(
            small.to_string(),
            FontId::proportional(font),
            theme::SOFT_INK,
        )
        .size()
        .x > hole * 0.78
    {
        font -= 0.5;
    }
    p.text(
        c + Vec2::new(0.0, size * 0.14),
        Align2::CENTER_CENTER,
        small,
        FontId::proportional(font),
        theme::SOFT_INK,
    );
    resp
}

/// A small filled circle with a check (done) or an open ring (in progress).
pub fn status_dot(ui: &mut Ui, done: bool, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    if done {
        p.circle_filled(rect.center(), size / 2.0, color);
        icons::paint(p, rect.shrink(size * 0.24), Icon::Check, Color32::WHITE);
    } else {
        p.circle_stroke(rect.center(), size / 2.0 - 1.5, Stroke::new(2.0, color));
    }
}

/// A numbered circle for step-by-step cards.
pub fn step_badge(ui: &mut Ui, n: usize) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(60.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 30.0, theme::ACCENT_SOFT);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        n.to_string(),
        theme::heading(26.0),
        theme::ACCENT,
    );
}

/// A pale yellow tip box with a light bulb.
pub fn tip(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::YELLOW_SOFT)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                icons::show(ui, Icon::Bulb, 22.0, theme::ORANGE);
                // A vertical column takes the rest of the width so the text wraps.
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(theme::glyphs(text))
                            .size(14.5)
                            .color(theme::SOFT_INK),
                    );
                });
            });
        });
}

// ---------------------------------------------------------------- page header

/// The big page title with a small label above, a line of explanation below, and the
/// painted landscape on the right.
pub fn page_header(ui: &mut Ui, kicker: &str, title: &str, subtitle: &str) {
    let width = ui.available_width();
    let height = 168.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let art_w = (width * 0.42).clamp(260.0, 520.0);
    let art = Rect::from_min_max(
        Pos2::new(rect.right() - art_w, rect.top() - 16.0),
        Pos2::new(rect.right() + 8.0, rect.bottom() + 8.0),
    );
    if width > 820.0 {
        header_art(ui.painter(), art);
    }
    let p = ui.painter();
    let x = rect.left() + 4.0;
    p.text(
        Pos2::new(x, rect.top() + 14.0),
        Align2::LEFT_CENTER,
        kicker.to_uppercase(),
        FontId::proportional(13.5),
        theme::SOFT_INK,
    );
    p.text(
        Pos2::new(x - 2.0, rect.top() + 58.0),
        Align2::LEFT_CENTER,
        theme::glyphs(title),
        theme::heading(42.0),
        theme::INK,
    );
    let wrap = (width - if width > 820.0 { art_w * 0.75 } else { 0.0 }).max(320.0);
    let galley = p.layout(
        subtitle.to_string(),
        FontId::proportional(18.0),
        theme::SOFT_INK,
        wrap,
    );
    p.galley(Pos2::new(x, rect.top() + 92.0), galley, theme::SOFT_INK);
}

/// A hill outline: a smooth wave across `rect` at height fraction `y`, filled to the bottom.
fn hill(p: &egui::Painter, r: Rect, y: f32, amp: f32, phase: f32, waves: f32, fill: Color32) {
    let mut pts = vec![Pos2::new(r.left(), r.bottom())];
    let steps = 48;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = r.left() + t * r.width();
        let yy = r.top() + r.height() * y - amp * (t * waves * std::f32::consts::TAU + phase).sin();
        pts.push(Pos2::new(x, yy));
    }
    pts.push(Pos2::new(r.right(), r.bottom()));
    // Triangulate as vertical strips (the hill is a height field).
    let mut mesh = Mesh::default();
    for w in pts[1..pts.len() - 1].windows(2) {
        let base = mesh.vertices.len() as u32;
        for q in [
            w[0],
            w[1],
            Pos2::new(w[1].x, r.bottom()),
            Pos2::new(w[0].x, r.bottom()),
        ] {
            mesh.vertices.push(Vertex {
                pos: q,
                uv: egui::epaint::WHITE_UV,
                color: fill,
            });
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    p.add(Shape::mesh(mesh));
}

fn leaf(p: &egui::Painter, base: Pos2, angle: f32, len: f32, fill: Color32) {
    let dir = Vec2::angled(angle);
    let side = Vec2::new(-dir.y, dir.x);
    let tip = base + dir * len;
    let mid = base + dir * len * 0.45;
    let pts = vec![base, mid + side * len * 0.22, tip, mid - side * len * 0.22];
    p.add(Shape::convex_polygon(pts, fill, Stroke::NONE));
    p.line_segment(
        [base, tip - dir * len * 0.1],
        Stroke::new(1.0, fill.gamma_multiply(0.7)),
    );
}

/// A soft landscape: sun, hills, leaves, and an acoustic guitar leaning in from the right.
/// It fades into the page on its left edge.
pub fn header_art(p: &egui::Painter, r: Rect) {
    let p = &p.with_clip_rect(r);
    let u = r.height() / 200.0;
    // Sun.
    p.circle_filled(
        Pos2::new(r.right() - 190.0 * u, r.top() + 62.0 * u),
        40.0 * u,
        Color32::from_rgb(0xF9, 0xCD, 0xA4),
    );
    // Hills, back to front.
    hill(
        p,
        r,
        0.62,
        18.0 * u,
        0.4,
        1.2,
        Color32::from_rgb(0xF7, 0xE3, 0xCF),
    );
    hill(
        p,
        r,
        0.70,
        14.0 * u,
        2.1,
        1.6,
        Color32::from_rgb(0xD9, 0xE4, 0xD8),
    );
    hill(
        p,
        r,
        0.80,
        10.0 * u,
        1.0,
        1.1,
        Color32::from_rgb(0xF3, 0xD6, 0xB6),
    );
    hill(
        p,
        r,
        0.92,
        6.0 * u,
        3.0,
        2.0,
        Color32::from_rgb(0xF6, 0xE2, 0xCB),
    );
    // Plants.
    let green = Color32::from_rgb(0x5F, 0x95, 0x78);
    let green2 = Color32::from_rgb(0x8C, 0xB8, 0x98);
    let orange = Color32::from_rgb(0xE3, 0x8A, 0x4F);
    let plants = [
        (0.30, green, 1.0),
        (0.38, orange, 0.7),
        (0.58, green2, 0.8),
        (0.97, green, 1.3),
    ];
    for (fx, color, scale) in plants {
        let base = Pos2::new(r.left() + r.width() * fx, r.bottom() - 6.0 * u);
        for (k, a) in [-2.2f32, -1.9, -1.57, -1.25, -0.95].iter().enumerate() {
            let len = (34.0 + (k % 3) as f32 * 10.0) * u * scale;
            leaf(
                p,
                base,
                *a,
                len,
                if k % 2 == 0 {
                    color
                } else {
                    color.gamma_multiply(0.85)
                },
            );
        }
    }
    // Guitar: body low right, neck rising to the top right.
    let body = Pos2::new(r.right() - 95.0 * u, r.bottom() - 52.0 * u);
    let dir = Vec2::angled(-1.25);
    let side = Vec2::new(-dir.y, dir.x);
    let wood = Color32::from_rgb(0xF0, 0xA4, 0x55);
    let edge = Color32::from_rgb(0xC9, 0x77, 0x2E);
    let lower = body;
    let upper = body + dir * 46.0 * u;
    p.circle_filled(lower, 46.0 * u, edge);
    p.circle_filled(upper, 36.0 * u, edge);
    p.circle_filled(lower, 43.0 * u, wood);
    p.circle_filled(upper, 33.0 * u, wood);
    p.circle_filled(
        body + dir * 36.0 * u,
        12.0 * u,
        Color32::from_rgb(0x4A, 0x30, 0x1E),
    );
    // Neck and headstock.
    let neck_start = upper + dir * 20.0 * u;
    let neck_end = upper + dir * 130.0 * u;
    let nw = 7.0 * u;
    p.add(Shape::convex_polygon(
        vec![
            neck_start + side * nw,
            neck_end + side * nw,
            neck_end - side * nw,
            neck_start - side * nw,
        ],
        Color32::from_rgb(0x8A, 0x5A, 0x35),
        Stroke::NONE,
    ));
    let head_end = neck_end + dir * 30.0 * u;
    p.add(Shape::convex_polygon(
        vec![
            neck_end + side * 9.0 * u,
            head_end + side * 8.0 * u,
            head_end - side * 8.0 * u,
            neck_end - side * 9.0 * u,
        ],
        Color32::from_rgb(0x5C, 0x3A, 0x22),
        Stroke::NONE,
    ));
    for k in 0..3 {
        let at = neck_end + dir * (8.0 + k as f32 * 9.0) * u;
        p.circle_filled(
            at + side * 12.0 * u,
            2.6 * u,
            Color32::from_rgb(0xD9, 0xD2, 0xC8),
        );
        p.circle_filled(
            at - side * 12.0 * u,
            2.6 * u,
            Color32::from_rgb(0xD9, 0xD2, 0xC8),
        );
    }
    // Strings and bridge.
    for k in [-1.5f32, -0.5, 0.5, 1.5] {
        let off = side * k * 2.2 * u;
        p.line_segment(
            [lower - dir * 20.0 * u + off, neck_end + off],
            Stroke::new(0.8, Color32::from_rgb(0xF4, 0xEE, 0xE6)),
        );
    }
    p.add(Shape::convex_polygon(
        vec![
            lower - dir * 18.0 * u + side * 14.0 * u,
            lower - dir * 24.0 * u + side * 14.0 * u,
            lower - dir * 24.0 * u - side * 14.0 * u,
            lower - dir * 18.0 * u - side * 14.0 * u,
        ],
        Color32::from_rgb(0x5C, 0x3A, 0x22),
        Stroke::NONE,
    ));
    // Fade the left edge into the page.
    let fade = Rect::from_min_max(r.min, Pos2::new(r.left() + r.width() * 0.35, r.bottom()));
    let mut mesh = Mesh::default();
    let solid = theme::BG;
    let clear = Color32::from_rgba_unmultiplied(solid.r(), solid.g(), solid.b(), 0);
    for (pos, color) in [
        (fade.left_top(), solid),
        (fade.right_top(), clear),
        (fade.right_bottom(), clear),
        (fade.left_bottom(), solid),
    ] {
        mesh.vertices.push(Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
}
