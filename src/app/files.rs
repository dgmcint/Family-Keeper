//! Opening, saving and exporting family files, and saving pictures.

use super::*;
use crate::data::*;
use crate::formats::fkf;
use eframe::egui::{self, ViewportCommand};
use std::path::PathBuf;

impl FamilyApp {
    pub(crate) fn install(&mut self, db: Database, path: Option<PathBuf>) {
        self.db = db;
        self.path = path;
        self.dirty = false;
        self.is_sample = false;
        self.undo.clear();
        self.redo.clear();
        self.history.clear();
        self.textures.clear();
        self.family_choice.clear();
        self.ignored_duplicates.clear();
        self.search.clear();
        self.revision += 1;
        self.duplicates = None;
        self.selected = self.db.home_person();
        self.tab = Tab::Family;
    }

    pub fn load(&mut self, path: &std::path::Path) {
        match fkf::open(path) {
            Ok(loaded) => {
                let imported = loaded.imported;
                let warnings = loaded.warnings;
                self.install(loaded.db, loaded.path);
                self.status = format!("Opened {}", path.file_name().unwrap_or_default().to_string_lossy());
                if imported {
                    self.dirty = true;
                    let mut text = format!(
                        "Brought in {} people, {} families and {} sources. The original GEDCOM file wasn't changed.\n\nChoose File, then Save, to keep this as a family file.",
                        self.db.people.len(),
                        self.db.families.len(),
                        self.db.sources.len()
                    );
                    if !warnings.is_empty() {
                        text.push_str("\n\nThings to check:");
                        for w in warnings.iter().take(20) {
                            text.push_str("\n• ");
                            text.push_str(w);
                        }
                        if warnings.len() > 20 {
                            text.push_str(&format!("\n• and {} more", warnings.len() - 20));
                        }
                    }
                    self.message = Some(("Import finished".into(), text));
                }
            }
            Err(e) => self.message = Some(("Couldn't open that file".into(), e)),
        }
    }

    pub(crate) fn run(&mut self, pending: Pending, ctx: &egui::Context) {
        match pending {
            Pending::NewFile => {
                self.install(Database::default(), None);
                self.status = "Started a new family file".into();
            }
            Pending::Open => {
                let picked = rfd::FileDialog::new()
                    .set_title("Open a family file")
                    .add_filter("Family files and GEDCOM", &[fkf::FILE_EXTENSION, fkf::OLD_EXTENSION, "ged", "GED", "gedcom"])
                    .pick_file();
                if let Some(path) = picked {
                    self.load(&path);
                }
            }
            Pending::Import => {
                let picked = rfd::FileDialog::new()
                    .set_title("Import a GEDCOM file")
                    .add_filter("GEDCOM", &["ged", "GED", "gedcom"])
                    .pick_file();
                if let Some(path) = picked {
                    self.load(&path);
                }
            }
            Pending::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    pub fn request(&mut self, pending: Pending, ctx: &egui::Context) {
        if self.dirty {
            self.confirm = Some(Confirm::Unsaved(pending));
        } else {
            self.run(pending, ctx);
        }
    }

    pub fn save(&mut self) -> bool {
        match self.path.clone() {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) -> bool {
        let picked = rfd::FileDialog::new()
            .set_title("Save the family file")
            .set_file_name(format!("Family.{}", fkf::FILE_EXTENSION))
            .add_filter("Family file", &[fkf::FILE_EXTENSION])
            .save_file();
        match picked {
            Some(path) => self.save_to(with_extension(path, fkf::FILE_EXTENSION)),
            None => false,
        }
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        match fkf::save(&path, &self.db) {
            Ok(()) => {
                self.status = format!("Saved {}", path.file_name().unwrap_or_default().to_string_lossy());
                self.path = Some(path);
                self.dirty = false;
                self.is_sample = false;
                true
            }
            Err(e) => {
                self.message = Some(("Couldn't save".into(), e));
                false
            }
        }
    }

    pub fn export_gedcom(&mut self) {
        let picked = rfd::FileDialog::new()
            .set_title("Export as GEDCOM")
            .set_file_name("Family.ged")
            .add_filter("GEDCOM", &["ged"])
            .save_file();
        if let Some(path) = picked {
            let path = with_extension(path, "ged");
            match fkf::write_safely(&path, crate::formats::gedcom::export(&self.db).as_bytes()) {
                Ok(()) => self.status = format!("Exported {}", path.file_name().unwrap_or_default().to_string_lossy()),
                Err(e) => self.message = Some(("Couldn't export".into(), e)),
            }
        }
    }
}

pub(crate) fn write_picture(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension(format!("{}.png", std::process::id()));
    let mut last = String::new();
    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        match std::fs::write(&temp, bytes).and_then(|()| std::fs::rename(&temp, path)) {
            Ok(()) => return Ok(()),
            Err(e) => last = e.to_string(),
        }
    }
    let _ = std::fs::remove_file(&temp);
    Err(last)
}

pub(crate) fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    if path.extension().is_some() { path } else { path.with_extension(ext) }
}
