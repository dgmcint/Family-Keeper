//! Family Keeper. Where things are:
//!
//! - `app`: the main window, its state, undo and redo, file handling and commands
//! - `screens`: the Family, Pedigree, list, Duplicates and Reports screens
//! - `dialogs`: the windows that open over the main screen
//! - `data`: the family file and the records in it
//! - `formats`: Family Keeper's own .fkf files and GEDCOM
//! - `reports`: the printable reports

// No black console window behind the program on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod data;
mod dialogs;
mod formats;
mod reports;
mod screens;

use app::colors;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut screenshot = None;
    let mut open = None;
    let mut show = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            let Some(file) = args.next() else {
                eprintln!("--screenshot needs a file name, for example: --screenshot picture.png");
                std::process::exit(2);
            };
            screenshot = Some(PathBuf::from(file));
        } else if arg == "--colors" {
            // Starts in a colour scheme by name, for pictures: Paper, Navy, Leather...
            if let Some(name) = args.next() {
                colors::set_name(&name.to_string_lossy());
            }
        } else if arg == "--show" {
            // Starts on a given screen: pedigree, people, reports, edit-person...
            show = args.next().map(|s| s.to_string_lossy().into_owned());
        } else if !arg.to_string_lossy().starts_with("--") {
            open = Some(PathBuf::from(arg));
        }
    }

    // A picture run must always end, whatever happens to the window.
    if screenshot.is_some() {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(180));
            eprintln!("Gave up waiting for the window to draw.");
            std::process::exit(3);
        });
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Family Keeper")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([820.0, 560.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/window-icon.png"))
                    .expect("window icon is a valid PNG"),
            ),
        persist_window: screenshot.is_none(),
        ..Default::default()
    };
    eframe::run_native(
        "Family Keeper",
        options,
        Box::new(move |cc| {
            let mut family = app::FamilyApp::new(cc, screenshot, open);
            if let Some(what) = show {
                family.show_screen(&what);
            }
            Ok(Box::new(family))
        }),
    )
}
