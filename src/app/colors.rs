//! The look of the program. Paper is the original warm paper with deep green; the others
//! follow the tree pictures. Chosen under View > Colors.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Every colour the screens use, by the job it does.
pub struct Palette {
    /// The page behind everything.
    pub paper: Color32,
    /// Raised cards and panels.
    pub card: Color32,
    /// A card under the pointer.
    pub card_hover: Color32,
    pub border: Color32,
    /// Links, highlighted edges and the selected person.
    pub accent: Color32,
    pub accent_soft: Color32,
    /// Unsaved-change notices.
    pub gold: Color32,
    pub gold_soft: Color32,
    pub ink: Color32,
    pub muted: Color32,
    /// Inside typing boxes.
    pub text_bg: Color32,
    pub faint: Color32,
    /// Buttons at rest and under the pointer.
    pub button: Color32,
    pub button_hover: Color32,
    pub dark: bool,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// `a` moved `pct` percent of the way to `b`.
const fn mix(a: Color32, b: Color32, pct: u32) -> Color32 {
    const fn ch(x: u8, y: u8, pct: u32) -> u8 {
        ((x as u32 * (100 - pct) + y as u32 * pct) / 100) as u8
    }
    rgb(ch(a.r(), b.r(), pct), ch(a.g(), b.g(), pct), ch(a.b(), b.b(), pct))
}

/// A scheme with a dark background and a light colour drawn on it, like the tree pictures.
const fn dark(base: Color32, cream: Color32) -> Palette {
    let black = rgb(0, 0, 0);
    let gold = rgb(236, 196, 120);
    Palette {
        paper: base,
        card: mix(base, cream, 9),
        card_hover: mix(base, cream, 16),
        border: mix(base, cream, 28),
        accent: cream,
        accent_soft: mix(base, cream, 26),
        gold,
        gold_soft: mix(base, gold, 22),
        ink: cream,
        muted: mix(cream, base, 32),
        text_bg: mix(base, black, 28),
        faint: mix(base, cream, 5),
        button: mix(base, cream, 14),
        button_hover: mix(base, cream, 22),
        dark: true,
    }
}

/// A scheme with a light background and dark text.
const fn light(paper: Color32, ink: Color32, accent: Color32) -> Palette {
    let white = rgb(255, 255, 255);
    let gold = rgb(0x8F, 0x6C, 0x1F);
    Palette {
        paper,
        card: mix(paper, white, 75),
        card_hover: mix(paper, white, 50),
        border: mix(paper, ink, 18),
        accent,
        accent_soft: mix(mix(paper, white, 60), accent, 18),
        gold,
        gold_soft: mix(mix(paper, white, 60), gold, 15),
        ink,
        muted: mix(ink, paper, 40),
        text_bg: mix(paper, white, 90),
        faint: mix(paper, ink, 4),
        button: mix(paper, ink, 6),
        button_hover: mix(paper, ink, 11),
        dark: false,
    }
}

/// Name, the group it is listed under, and its colours.
pub const SCHEMES: &[(&str, &str, Palette)] = &[
    (
        "Paper",
        "Original",
        Palette {
            paper: rgb(0xF5, 0xF0, 0xE6),
            card: rgb(0xFF, 0xFD, 0xF8),
            card_hover: rgb(0xF4, 0xEF, 0xE2),
            border: rgb(0xD9, 0xCF, 0xBC),
            accent: rgb(0x2F, 0x6B, 0x57),
            accent_soft: rgb(0xDD, 0xEB, 0xE3),
            gold: rgb(0x8F, 0x6C, 0x1F),
            gold_soft: rgb(0xF6, 0xEC, 0xD2),
            ink: rgb(0x2A, 0x26, 0x20),
            muted: rgb(0x66, 0x5E, 0x52),
            text_bg: rgb(0xFF, 0xFF, 0xFF),
            faint: rgb(0xEF, 0xE9, 0xDC),
            button: rgb(0xEC, 0xE5, 0xD5),
            button_hover: rgb(0xE3, 0xDB, 0xC8),
            dark: false,
        },
    ),
    ("Slate Blue", "Tree pictures", dark(rgb(67, 82, 106), rgb(246, 238, 226))),
    ("Navy", "Tree pictures", dark(rgb(30, 52, 99), rgb(255, 255, 255))),
    (
        "Parchment",
        "Tree pictures",
        Palette {
            paper: rgb(226, 201, 162),
            card: rgb(247, 236, 214),
            card_hover: rgb(240, 225, 198),
            border: rgb(186, 150, 104),
            accent: rgb(122, 72, 30),
            accent_soft: rgb(236, 212, 174),
            gold: rgb(140, 90, 20),
            gold_soft: rgb(242, 224, 188),
            ink: rgb(66, 40, 18),
            muted: rgb(112, 82, 52),
            text_bg: rgb(252, 246, 234),
            faint: rgb(233, 213, 180),
            button: rgb(238, 220, 190),
            button_hover: rgb(230, 208, 172),
            dark: false,
        },
    ),
    ("Blue Linen", "Tree pictures", dark(rgb(88, 110, 142), rgb(236, 231, 220))),
    ("Leather", "Tree pictures", dark(rgb(104, 52, 24), rgb(242, 224, 188))),
    ("Leaf Green", "Genealogy sites", light(rgb(243, 240, 232), rgb(52, 48, 44), rgb(104, 134, 28))),
    ("Fresh Green", "Genealogy sites", light(rgb(242, 243, 240), rgb(51, 51, 51), rgb(80, 132, 34))),
    ("Deep Green", "Genealogy sites", dark(rgb(48, 88, 38), rgb(250, 252, 246))),
    ("Sage", "Earth tones", light(rgb(199, 208, 184), rgb(40, 52, 36), rgb(88, 110, 74))),
    ("Olive", "Earth tones", dark(rgb(85, 90, 52), rgb(240, 236, 214))),
    ("Moss", "Earth tones", dark(rgb(66, 82, 50), rgb(238, 240, 220))),
    ("Forest Green", "Earth tones", dark(rgb(36, 66, 48), rgb(234, 240, 226))),
    ("Terracotta", "Earth tones", dark(rgb(150, 78, 52), rgb(250, 236, 222))),
    ("Clay", "Earth tones", light(rgb(214, 180, 156), rgb(70, 40, 28), rgb(150, 80, 50))),
    ("Burgundy", "Popular colors", dark(rgb(110, 28, 40), rgb(250, 236, 236))),
    ("Teal", "Popular colors", dark(rgb(22, 92, 100), rgb(232, 246, 244))),
    ("Plum", "Popular colors", dark(rgb(82, 46, 88), rgb(244, 234, 246))),
    ("Charcoal", "Popular colors", dark(rgb(52, 54, 58), rgb(236, 236, 232))),
    ("Midnight", "Popular colors", dark(rgb(20, 24, 40), rgb(226, 230, 240))),
    ("Sky", "Popular colors", light(rgb(200, 222, 240), rgb(20, 44, 70), rgb(40, 100, 170))),
    ("Rose", "Popular colors", light(rgb(236, 210, 212), rgb(80, 30, 40), rgb(160, 60, 80))),
    ("Ivory", "Neutrals", light(rgb(246, 242, 228), rgb(40, 38, 30), rgb(110, 104, 80))),
    ("Linen", "Neutrals", light(rgb(236, 230, 218), rgb(50, 44, 36), rgb(120, 100, 70))),
    ("Stone", "Neutrals", light(rgb(214, 212, 206), rgb(40, 40, 38), rgb(90, 94, 100))),
    ("Taupe", "Neutrals", light(rgb(200, 190, 178), rgb(50, 42, 36), rgb(110, 92, 76))),
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// The colours in use now.
pub fn pal() -> &'static Palette {
    &SCHEMES[current()].2
}

pub fn current() -> usize {
    CURRENT.load(Ordering::Relaxed).min(SCHEMES.len() - 1)
}

/// The scheme's name as saved in the settings, so a reordered list still finds it.
pub fn name() -> &'static str {
    SCHEMES[current()].0
}

pub fn set_name(name: &str) {
    if let Some(i) = SCHEMES.iter().position(|(n, _, _)| n.eq_ignore_ascii_case(name.trim())) {
        CURRENT.store(i, Ordering::Relaxed);
    }
}

/// Switches to a scheme and redraws in it.
pub fn choose(ctx: &egui::Context, index: usize) {
    CURRENT.store(index.min(SCHEMES.len() - 1), Ordering::Relaxed);
    apply(ctx);
}

pub fn apply(ctx: &egui::Context) {
    let p = pal();
    ctx.set_theme(egui::ThemePreference::Light);
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.paper;
    v.window_fill = p.card;
    v.extreme_bg_color = p.text_bg;
    v.faint_bg_color = p.faint;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.warn_fg_color = p.gold;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.ink);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    let radius = CornerRadius::same(6);
    let button = |w: &mut egui::style::WidgetVisuals, fill: Color32, edge: Color32| {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, edge);
        w.fg_stroke = Stroke::new(1.0, p.ink);
        w.corner_radius = radius;
    };
    button(&mut v.widgets.inactive, p.button, p.border);
    button(&mut v.widgets.hovered, p.button_hover, p.accent);
    button(&mut v.widgets.active, p.accent_soft, p.accent);
    button(&mut v.widgets.open, p.accent_soft, p.accent);
    // The program always runs in its light setting, so this one set of colours is the one shown.
    ctx.set_visuals_of(egui::Theme::Light, v);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 26.0;
        for (kind, size, family) in [
            (TextStyle::Body, 15.0, FontFamily::Proportional),
            (TextStyle::Button, 15.0, FontFamily::Proportional),
            (TextStyle::Small, 12.0, FontFamily::Proportional),
            (TextStyle::Heading, 21.0, FontFamily::Proportional),
            (TextStyle::Monospace, 14.0, FontFamily::Monospace),
        ] {
            style.text_styles.insert(kind, FontId::new(size, family));
        }
    });
}
