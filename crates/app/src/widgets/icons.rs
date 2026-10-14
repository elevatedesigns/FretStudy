//! Line icons drawn with the painter, so they stay crisp at any size and take any color.

use egui::epaint::{CubicBezierShape, PathShape, PathStroke};
use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Map,
    Guitar,
    Search,
    Target,
    Speaker,
    Gear,
    Stopwatch,
    Music,
    Play,
    Stop,
    Mic,
    Refresh,
    Star,
    Book,
    Bulb,
    ArrowDown,
    ArrowUp,
    Check,
    ChevronRight,
    ChevronLeft,
    ChevronDown,
    ChevronUp,
    Headphones,
    Metronome,
    Trash,
    Users,
    Download,
    Upload,
    Ear,
    Grid,
    Lines,
    Lock,
    Hand,
}

/// Draws an icon inside `rect` (square works best).
pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let w = (s * 0.085).max(1.4);
    let st = Stroke::new(w, color);
    // Points in a -1..1 box around the center.
    let at = |x: f32, y: f32| Pos2::new(c.x + x * s * 0.5, c.y + y * s * 0.5);
    let line = |pts: &[(f32, f32)]| {
        p.add(Shape::line(
            pts.iter().map(|(x, y)| at(*x, *y)).collect(),
            st,
        ));
    };
    let closed = |pts: &[(f32, f32)]| {
        p.add(Shape::Path(PathShape::closed_line(
            pts.iter().map(|(x, y)| at(*x, *y)).collect(),
            st,
        )));
    };
    let curve = |a: (f32, f32), b: (f32, f32), cc: (f32, f32), d: (f32, f32)| {
        p.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
            [at(a.0, a.1), at(b.0, b.1), at(cc.0, cc.1), at(d.0, d.1)],
            false,
            Color32::TRANSPARENT,
            PathStroke::new(w, color),
        )));
    };
    let r = |f: f32| f * s * 0.5;
    match icon {
        Icon::Map => {
            closed(&[
                (-0.85, -0.6),
                (-0.3, -0.8),
                (0.3, -0.6),
                (0.85, -0.8),
                (0.85, 0.6),
                (0.3, 0.8),
                (-0.3, 0.6),
                (-0.85, 0.8),
            ]);
            line(&[(-0.3, -0.8), (-0.3, 0.6)]);
            line(&[(0.3, -0.6), (0.3, 0.8)]);
        }
        Icon::Guitar => {
            // Body (two circles) on a diagonal, with a neck going up-right.
            p.circle_stroke(at(-0.42, 0.42), r(0.36), st);
            p.circle_stroke(at(-0.12, 0.12), r(0.26), st);
            p.circle_filled(at(-0.33, 0.33), r(0.09), color);
            line(&[(-0.05, 0.05), (0.62, -0.62)]);
            line(&[(0.55, -0.75), (0.8, -0.5)]);
            line(&[(0.62, -0.62), (0.88, -0.88)]);
        }
        Icon::Search => {
            p.circle_stroke(at(-0.15, -0.15), r(0.5), st);
            line(&[(0.22, 0.22), (0.8, 0.8)]);
        }
        Icon::Target => {
            p.circle_stroke(c, r(0.82), st);
            p.circle_stroke(c, r(0.48), st);
            p.circle_filled(c, r(0.14), color);
            line(&[(0.0, 0.0), (0.7, -0.7)]);
        }
        Icon::Speaker => {
            closed(&[
                (-0.85, -0.3),
                (-0.45, -0.3),
                (0.0, -0.75),
                (0.0, 0.75),
                (-0.45, 0.3),
                (-0.85, 0.3),
            ]);
            curve((0.3, -0.35), (0.48, -0.15), (0.48, 0.15), (0.3, 0.35));
            curve((0.5, -0.65), (0.85, -0.3), (0.85, 0.3), (0.5, 0.65));
        }
        Icon::Gear => {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let (sn, cs) = a.sin_cos();
                line(&[(cs * 0.58, sn * 0.58), (cs * 0.86, sn * 0.86)]);
            }
            p.circle_stroke(c, r(0.6), st);
            p.circle_stroke(c, r(0.24), st);
        }
        Icon::Stopwatch => {
            p.circle_stroke(at(0.0, 0.12), r(0.72), st);
            line(&[(-0.2, -0.88), (0.2, -0.88)]);
            line(&[(0.0, -0.88), (0.0, -0.6)]);
            line(&[(0.0, 0.12), (0.28, -0.2)]);
        }
        Icon::Music => {
            line(&[(-0.3, 0.55), (-0.3, -0.75), (0.65, -0.95), (0.65, 0.35)]);
            p.circle_filled(at(-0.5, 0.58), r(0.22), color);
            p.circle_filled(at(0.45, 0.38), r(0.22), color);
        }
        Icon::Play => {
            p.add(Shape::convex_polygon(
                vec![at(-0.45, -0.7), at(0.7, 0.0), at(-0.45, 0.7)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Stop => {
            p.rect_filled(
                Rect::from_center_size(c, Vec2::splat(r(1.2))),
                egui::CornerRadius::same((s * 0.08) as u8),
                color,
            );
        }
        Icon::Mic => {
            p.rect_stroke(
                Rect::from_center_size(at(0.0, -0.3), Vec2::new(r(0.6), r(1.0))),
                egui::CornerRadius::same((s * 0.15) as u8),
                st,
                egui::StrokeKind::Middle,
            );
            curve((-0.55, -0.1), (-0.55, 0.55), (0.55, 0.55), (0.55, -0.1));
            line(&[(0.0, 0.42), (0.0, 0.85)]);
            line(&[(-0.3, 0.85), (0.3, 0.85)]);
        }
        Icon::Refresh => {
            curve((0.7, -0.15), (0.6, -0.8), (-0.45, -0.85), (-0.7, -0.15));
            curve((-0.7, 0.15), (-0.6, 0.8), (0.45, 0.85), (0.7, 0.15));
            line(&[(0.45, -0.25), (0.72, -0.12), (0.85, -0.42)]);
            line(&[(-0.45, 0.25), (-0.72, 0.12), (-0.85, 0.42)]);
        }
        Icon::Star => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let rr = if i % 2 == 0 { 0.88 } else { 0.38 };
                    (a.cos() * rr, a.sin() * rr + 0.06)
                })
                .collect();
            closed(&pts);
        }
        Icon::Book => {
            closed(&[
                (0.0, -0.55),
                (-0.85, -0.75),
                (-0.85, 0.6),
                (0.0, 0.8),
                (0.85, 0.6),
                (0.85, -0.75),
            ]);
            line(&[(0.0, -0.55), (0.0, 0.8)]);
        }
        Icon::Bulb => {
            p.circle_stroke(at(0.0, -0.25), r(0.55), st);
            line(&[(-0.25, 0.3), (-0.25, 0.6), (0.25, 0.6), (0.25, 0.3)]);
            line(&[(-0.18, 0.82), (0.18, 0.82)]);
        }
        Icon::ArrowDown => {
            line(&[(0.0, -0.8), (0.0, 0.75)]);
            line(&[(-0.55, 0.2), (0.0, 0.78), (0.55, 0.2)]);
        }
        Icon::ArrowUp => {
            line(&[(0.0, 0.8), (0.0, -0.75)]);
            line(&[(-0.55, -0.2), (0.0, -0.78), (0.55, -0.2)]);
        }
        Icon::Check => line(&[(-0.65, 0.0), (-0.18, 0.5), (0.7, -0.5)]),
        Icon::ChevronRight => line(&[(-0.25, -0.6), (0.35, 0.0), (-0.25, 0.6)]),
        Icon::ChevronLeft => line(&[(0.25, -0.6), (-0.35, 0.0), (0.25, 0.6)]),
        Icon::ChevronDown => line(&[(-0.6, -0.25), (0.0, 0.35), (0.6, -0.25)]),
        Icon::ChevronUp => line(&[(-0.6, 0.25), (0.0, -0.35), (0.6, 0.25)]),
        Icon::Headphones => {
            curve((-0.75, 0.3), (-0.85, -0.95), (0.85, -0.95), (0.75, 0.3));
            p.rect_filled(
                Rect::from_center_size(at(-0.66, 0.42), Vec2::new(r(0.3), r(0.62))),
                egui::CornerRadius::same(3),
                color,
            );
            p.rect_filled(
                Rect::from_center_size(at(0.66, 0.42), Vec2::new(r(0.3), r(0.62))),
                egui::CornerRadius::same(3),
                color,
            );
        }
        Icon::Metronome => {
            closed(&[(-0.2, -0.85), (0.2, -0.85), (0.62, 0.8), (-0.62, 0.8)]);
            line(&[(-0.5, 0.35), (0.5, 0.35)]);
            line(&[(0.0, 0.35), (0.35, -0.55)]);
        }
        Icon::Trash => {
            line(&[(-0.75, -0.55), (0.75, -0.55)]);
            line(&[(-0.25, -0.55), (-0.2, -0.82), (0.2, -0.82), (0.25, -0.55)]);
            closed(&[(-0.55, -0.55), (-0.45, 0.82), (0.45, 0.82), (0.55, -0.55)]);
            line(&[(-0.15, -0.25), (-0.15, 0.5)]);
            line(&[(0.15, -0.25), (0.15, 0.5)]);
        }
        Icon::Users => {
            p.circle_stroke(at(-0.25, -0.35), r(0.3), st);
            curve((-0.85, 0.8), (-0.85, 0.05), (0.35, 0.05), (0.35, 0.8));
            p.circle_stroke(at(0.42, -0.42), r(0.24), st);
            curve((0.45, 0.0), (0.75, 0.0), (0.9, 0.25), (0.9, 0.6));
        }
        Icon::Download => {
            line(&[(0.0, -0.85), (0.0, 0.35)]);
            line(&[(-0.45, -0.05), (0.0, 0.38), (0.45, -0.05)]);
            line(&[(-0.8, 0.45), (-0.8, 0.82), (0.8, 0.82), (0.8, 0.45)]);
        }
        Icon::Upload => {
            line(&[(0.0, 0.35), (0.0, -0.85)]);
            line(&[(-0.45, -0.45), (0.0, -0.88), (0.45, -0.45)]);
            line(&[(-0.8, 0.45), (-0.8, 0.82), (0.8, 0.82), (0.8, 0.45)]);
        }
        Icon::Ear => {
            curve((-0.45, -0.1), (-0.55, -0.95), (0.6, -0.95), (0.5, -0.1));
            curve((0.5, -0.1), (0.45, 0.35), (0.0, 0.3), (0.0, 0.7));
            curve((0.0, 0.7), (0.0, 0.95), (-0.35, 0.95), (-0.4, 0.7));
            curve((-0.15, -0.15), (-0.15, -0.55), (0.25, -0.55), (0.22, -0.15));
        }
        Icon::Grid => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(r(1.3), r(1.6))),
                egui::CornerRadius::same(2),
                st,
                egui::StrokeKind::Middle,
            );
            for x in [-0.22, 0.22] {
                line(&[(x, -0.8), (x, 0.8)]);
            }
            for y in [-0.27, 0.27] {
                line(&[(-0.65, y), (0.65, y)]);
            }
        }
        Icon::Lines => {
            for (i, y) in [-0.6f32, -0.2, 0.2, 0.6].iter().enumerate() {
                let x1 = if i == 3 { 0.2 } else { 0.75 };
                line(&[(-0.75, *y), (x1, *y)]);
            }
        }
        Icon::Lock => {
            p.rect_stroke(
                Rect::from_center_size(at(0.0, 0.3), Vec2::new(r(1.3), r(1.0))),
                egui::CornerRadius::same(3),
                st,
                egui::StrokeKind::Middle,
            );
            curve((-0.42, -0.2), (-0.42, -0.95), (0.42, -0.95), (0.42, -0.2));
        }
        Icon::Hand => {
            for (i, x) in [-0.45f32, -0.15, 0.15, 0.45].iter().enumerate() {
                let top = [-0.45, -0.8, -0.8, -0.55][i];
                line(&[(*x, 0.1), (*x, top)]);
            }
            curve((-0.45, 0.1), (-0.5, 0.85), (0.55, 0.95), (0.45, 0.1));
            line(&[(-0.45, 0.25), (-0.8, -0.1)]);
        }
    }
}

/// An icon as its own widget (decorative unless the caller adds a label).
pub fn show(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint(ui.painter(), rect, icon, color);
    resp
}
