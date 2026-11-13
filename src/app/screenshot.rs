//! The `--screenshot` option: wait for the window to settle, save a picture and exit.

use super::*;
use eframe::egui::{self};

impl FamilyApp {
    pub(crate) fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        ctx.request_repaint();
        self.frames += 1;
        if self.frames >= 120 && !self.screenshot_sent {
            self.screenshot_sent = true;
            ctx.request_screenshot(move |image| {
                let [w, h] = image.size;
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), 255]).collect();
                if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
                    let _ = std::fs::create_dir_all(folder);
                }
                let saved = image::RgbaImage::from_raw(w as u32, h as u32, pixels)
                    .ok_or_else(|| "the picture had the wrong size".to_string())
                    .and_then(|img| {
                        let mut png = std::io::Cursor::new(Vec::new());
                        img.write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
                        write_picture(&path, png.get_ref())
                    });
                match saved {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("Couldn't save the picture: {e}");
                        std::process::exit(1);
                    }
                }
            });
        }
    }
}
