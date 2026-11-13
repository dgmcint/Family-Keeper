//! Fallback fonts for Chinese, Japanese and Korean text. The built-in font already covers Latin,
//! Greek and Cyrillic; these are only used for characters it doesn't have.
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

#[cfg(target_os = "macos")]
const WIDE_FONTS: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/Library/Fonts/Arial Unicode.ttf",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
];

#[cfg(target_os = "windows")]
const WIDE_FONTS: &[&str] = &[
    "C:\\Windows\\Fonts\\ARIALUNI.TTF",
    "C:\\Windows\\Fonts\\msjh.ttc",
    "C:\\Windows\\Fonts\\YuGothM.ttc",
    "C:\\Windows\\Fonts\\msgothic.ttc",
    "C:\\Windows\\Fonts\\malgun.ttf",
    "C:\\Windows\\Fonts\\msyh.ttc",
];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const WIDE_FONTS: &[&str] = &[
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
];

/// Adds the first wide font found on this computer behind the built-in fonts.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for path in WIDE_FONTS {
        let Ok(bytes) = std::fs::read(path) else { continue };
        fonts.font_data.insert("wide".into(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push("wide".into());
        }
        break;
    }
    ctx.set_fonts(fonts);
}
