//! Saving and opening family files. A save either finishes completely or leaves the old file alone.

use crate::data::Database;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE_EXTENSION: &str = "fkf";

/// The ending used before the program was renamed. Those files still open.
pub const OLD_EXTENSION: &str = "familyfile";

// Kept from the program's first name, Family Keeper, so files saved before the rename still open.
const FORMAT: &str = "family-file";

const VERSION: u32 = 1;

#[derive(Serialize)]
struct SavedRef<'a> {
    format: &'static str,
    version: u32,
    db: &'a Database,
}

#[derive(Deserialize)]
struct Saved {
    format: String,
    version: u32,
    db: Database,
}

pub struct Loaded {
    pub db: Database,
    /// Set when the file can be saved straight back to where it came from.
    pub path: Option<PathBuf>,
    pub warnings: Vec<String>,
    pub imported: bool,
}

/// Writes beside the target first, keeps the previous version as a backup, then swaps the new one in.
pub fn write_safely(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temp = path.with_file_name(format!("{name}.saving"));
    let attempt = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if path.exists() {
            std::fs::copy(path, path.with_file_name(format!("{name}.backup")))?;
        }
        std::fs::rename(&temp, path)
    })();
    attempt.map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("Couldn't save {name}: {e}. Your earlier copy wasn't changed.")
    })
}

pub fn save(path: &Path, db: &Database) -> Result<(), String> {
    let saved = SavedRef { format: FORMAT, version: VERSION, db };
    let bytes = serde_json::to_vec(&saved).map_err(|e| format!("Couldn't prepare the file: {e}"))?;
    write_safely(path, &bytes)
}

pub fn open(path: &Path) -> Result<Loaded, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    // Database files from other genealogy programs, which only that program can read.
    if matches!(ext.as_str(), "paf" | "ftw" | "ftm" | "ftmb" | "rmgc" | "rmtree" | "fbk" | "gw") {
        return Err(format!(
            "{name} is from another genealogy program, which this program can't read directly. In that program, choose to export it as a GEDCOM file, then import that here."
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't open {name}: {e}"))?;
    if ext == "ged" || ext == "gedcom" {
        let got = crate::formats::gedcom::import(&bytes).map_err(|e| format!("Couldn't import {name}. {e}"))?;
        return Ok(Loaded { db: got.db, path: None, warnings: got.warnings, imported: true });
    }
    let saved: Saved = serde_json::from_slice(&bytes)
        .map_err(|_| format!("{name} isn't a family file this program can read."))?;
    if saved.format != FORMAT {
        return Err(format!("{name} isn't a family file this program can read."));
    }
    if saved.version > VERSION {
        return Err(format!("{name} was saved by a newer version of this program. Please update it first."));
    }
    let mut db = saved.db;
    db.repair_links();
    Ok(Loaded { db, path: Some(path.to_path_buf()), warnings: vec![], imported: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("family-keeper-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn saved_file_opens_identically_and_keeps_a_backup() {
        let dir = temp_dir("save");
        let path = dir.join("mine.fkf");
        let db = Database::sample();
        save(&path, &Database::default()).unwrap();
        save(&path, &db).unwrap();
        assert_eq!(open(&path).unwrap().db, db);
        assert!(dir.join("mine.fkf.backup").exists());
        assert!(!dir.join("mine.fkf.saving").exists());
        let backup = serde_json::from_slice::<Saved>(&std::fs::read(dir.join("mine.fkf.backup")).unwrap()).unwrap();
        assert!(backup.db.people.is_empty());
    }

    #[test]
    fn explains_files_it_cant_read() {
        let dir = temp_dir("bad");
        let other_program = dir.join("old.paf");
        std::fs::write(&other_program, b"anything").unwrap();
        assert!(open(&other_program).err().unwrap().contains("GEDCOM"));
        let junk = dir.join("junk.fkf");
        std::fs::write(&junk, b"not json").unwrap();
        assert!(open(&junk).is_err());
        let other = dir.join("other.fkf");
        std::fs::write(&other, br#"{"format":"else","version":1,"db":{}}"#).unwrap();
        assert!(open(&other).is_err());
        let newer = dir.join("newer.fkf");
        std::fs::write(&newer, br#"{"format":"family-file","version":99,"db":{}}"#).unwrap();
        assert!(open(&newer).err().unwrap().contains("newer"));
        assert!(open(&dir.join("missing.fkf")).is_err());
    }

    #[test]
    fn failed_save_leaves_nothing_behind() {
        let dir = temp_dir("fail");
        let path = dir.join("no-such-folder").join("mine.fkf");
        assert!(save(&path, &Database::sample()).is_err());
    }
}
