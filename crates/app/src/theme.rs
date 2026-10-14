//! Colors, fonts, and the overall look. Colors come from the Okabe–Ito palette so they
//! stay distinguishable for color-blind learners. Color never carries meaning alone:
//! every dot also has a number or label, and roots/blue notes have their own shapes.

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use theory::Role;

// Okabe–Ito
pub const ORANGE: Color32 = Color32::from_rgb(0xE6, 0x9F, 0x00);
pub const SKY_BLUE: Color32 = Color32::from_rgb(0x56, 0xB4, 0xE9);
pub const GREEN: Color32 = Color32::from_rgb(0x00, 0x9E, 0x73);
pub const YELLOW: Color32 = Color32::from_rgb(0xF0, 0xE4, 0x42);
pub const BLUE: Color32 = Color32::from_rgb(0x00, 0x72, 0xB2);
pub const VERMILLION: Color32 = Color32::from_rgb(0xD5, 0x5E, 0x00);
pub const PURPLE: Color32 = Color32::from_rgb(0xCC, 0x79, 0xA7);
pub const GRAY: Color32 = Color32::from_rgb(0x6B, 0x6B, 0x6B);
pub const LIGHT_GRAY: Color32 = Color32::from_rgb(0xBD, 0xBD, 0xBD);

// App surfaces
pub const BG: Color32 = Color32::from_rgb(0xFB, 0xF7, 0xF1);
/// The sidebar and other raised surfaces.
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xFC, 0xF8);
/// Hairline borders around cards.
pub const BORDER: Color32 = Color32::from_rgb(0xEF, 0xE6, 0xD9);
/// The app's call-to-action orange (buttons, the active page, highlights).
pub const ACCENT: Color32 = Color32::from_rgb(0xEA, 0x55, 0x1C);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xCF, 0x47, 0x12);
/// A pale orange wash (selected rows, the active page in the sidebar).
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xFD, 0xEB, 0xDF);
/// Pale tints for finished / in-progress tiles.
pub const GREEN_SOFT: Color32 = Color32::from_rgb(0xE7, 0xF5, 0xEE);
pub const BLUE_SOFT: Color32 = Color32::from_rgb(0xE6, 0xF3, 0xFC);
pub const YELLOW_SOFT: Color32 = Color32::from_rgb(0xFE, 0xF6, 0xE0);
pub const PANEL: Color32 = Color32::from_rgb(0xF2, 0xEC, 0xE1);
pub const INK: Color32 = Color32::from_rgb(0x2A, 0x26, 0x22);
pub const SOFT_INK: Color32 = Color32::from_rgb(0x6E, 0x66, 0x5C);
pub const WOOD: Color32 = Color32::from_rgb(0xE9, 0xD8, 0xBC);
pub const WOOD_DARK: Color32 = Color32::from_rgb(0xC9, 0xB2, 0x8D);
pub const FRET: Color32 = Color32::from_rgb(0x9A, 0x96, 0x90);
pub const STRING: Color32 = Color32::from_rgb(0x5C, 0x55, 0x4C);
pub const NUT: Color32 = Color32::from_rgb(0x3A, 0x34, 0x2E);

/// Fretting-finger colors (Finger mode): 1 index … 4 pinky, 5 = thumb.
pub fn finger_color(finger: u8) -> Color32 {
    match finger {
        1 => BLUE,
        2 => GREEN,
        3 => ORANGE,
        4 => PURPLE,
        _ => GRAY,
    }
}

/// Note-job colors (Note-Role mode).
pub fn role_color(role: Role) -> Color32 {
    match role {
        Role::Root => VERMILLION,
        Role::Third => SKY_BLUE,
        Role::Fifth => GREEN,
        Role::Seventh => PURPLE,
        Role::Other => LIGHT_GRAY,
        Role::BlueNote => BLUE,
    }
}

/// Black or white, whichever reads better on `fill` (WCAG relative luminance).
pub fn text_on(fill: Color32) -> Color32 {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * lin(fill.r()) + 0.7152 * lin(fill.g()) + 0.0722 * lin(fill.b());
    // Contrast against white = 1.05/(l+0.05); against black = (l+0.05)/0.05.
    if 1.05 / (l + 0.05) >= (l + 0.05) / 0.05 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

static MUSIC_GLYPHS: AtomicBool = AtomicBool::new(false);

/// Whether a font with ♯ and ♭ was found. If not, labels use "#" and "b".
pub fn has_music_glyphs() -> bool {
    MUSIC_GLYPHS.load(Ordering::Relaxed)
}

/// Makes a label safe to draw with the loaded fonts.
pub fn glyphs(s: &str) -> String {
    if has_music_glyphs() {
        s.to_string()
    } else {
        s.replace('♯', "#")
            .replace('♭', "b")
            .replace('𝄪', "x")
            .replace('𝄫', "bb")
    }
}

pub fn heading_family() -> FontFamily {
    FontFamily::Name("Bold".into())
}

/// The friendly rounded face, used for the FretStudy logo.
pub fn logo_family() -> FontFamily {
    FontFamily::Name("Rounded".into())
}

/// The FretStudy wordmark font.
pub fn logo(size: f32) -> FontId {
    if HEADING_FONT.load(Ordering::Relaxed) {
        FontId::new(size, logo_family())
    } else {
        FontId::proportional(size)
    }
}

static HEADING_FONT: AtomicBool = AtomicBool::new(false);

/// Bold heading font (falls back to the body font until fonts are installed).
pub fn heading(size: f32) -> FontId {
    if HEADING_FONT.load(Ordering::Relaxed) {
        FontId::new(size, heading_family())
    } else {
        FontId::proportional(size)
    }
}

fn first_existing(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

/// Loads friendly system fonts when available (they are only read at runtime, never bundled).
/// Falls back to egui's built-in fonts. A bundled font (e.g. Atkinson Hyperlegible) can
/// replace this later.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let body = first_existing(&[
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ]);
    let rounded = first_existing(&[
        "/System/Library/Fonts/Supplemental/Arial Rounded Bold.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
    ]);
    let bold = first_existing(&[
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
    ]);
    let symbols = first_existing(&[
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Apple Symbols.ttf",
        "C:\\Windows\\Fonts\\seguisym.ttf",
    ]);

    if let Some(data) = body {
        fonts
            .font_data
            .insert("body".into(), Arc::new(FontData::from_owned(data)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "body".into());
    }
    if let Some(data) = symbols {
        fonts
            .font_data
            .insert("symbols".into(), Arc::new(FontData::from_owned(data)));
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(fam)
                .or_default()
                .push("symbols".into());
        }
        MUSIC_GLYPHS.store(true, Ordering::Relaxed);
    }
    let body_stack = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (family, name, data) in [
        (logo_family(), "rounded", rounded),
        (heading_family(), "bold", bold),
    ] {
        let mut stack: Vec<String> = Vec::new();
        if let Some(data) = data {
            fonts
                .font_data
                .insert(name.into(), Arc::new(FontData::from_owned(data)));
            stack.push(name.into());
        }
        stack.extend(body_stack.iter().cloned());
        fonts.families.insert(family, stack);
    }
    ctx.set_fonts(fonts);
    HEADING_FONT.store(true, Ordering::Relaxed);
}

/// The light, warm "Playful" look.
pub fn apply_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, heading_family())),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Small, FontId::proportional(13.0)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 32.0;
        let v = &mut style.visuals;
        *v = egui::Visuals::light();
        v.panel_fill = BG;
        v.window_fill = BG;
        v.extreme_bg_color = Color32::WHITE;
        v.override_text_color = Some(INK);
        v.selection.bg_fill = ACCENT_SOFT;
        v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        v.slider_trailing_fill = true;
        let r = egui::CornerRadius::same(10);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = r;
        }
        // Buttons: white with a hairline border, warming up on hover.
        v.widgets.inactive.weak_bg_fill = Color32::WHITE;
        v.widgets.inactive.bg_fill = Color32::from_rgb(0xF3, 0xEC, 0xE2);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.5, INK);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xFF, 0xF4, 0xEC);
        v.widgets.hovered.bg_fill = Color32::from_rgb(0xF6, 0xD9, 0xC6);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(0xF2, 0xB8, 0x95));
        v.widgets.active.weak_bg_fill = ACCENT_SOFT;
        v.widgets.active.bg_fill = ACCENT;
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_text_colors() {
        assert_eq!(text_on(BLUE), Color32::WHITE);
        assert_eq!(text_on(GREEN), Color32::BLACK);
        assert_eq!(text_on(YELLOW), Color32::BLACK);
        assert_eq!(text_on(SKY_BLUE), Color32::BLACK);
        assert_eq!(text_on(ORANGE), Color32::BLACK);
        assert_eq!(text_on(NUT), Color32::WHITE);
    }
}
