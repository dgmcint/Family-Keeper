//! Carrying out what the person asked for, and keeping the lists and reports up to date.

use super::*;
use crate::dialogs::{self, FamilyEditor, PersonEditor, Picker};
use crate::data::*;
use crate::reports;
use crate::formats::fkf;
use eframe::egui::{self};
use std::collections::HashMap;

impl FamilyApp {
    pub(crate) fn refresh_rows(&mut self) {
        let sort_code = self.sort as u8;
        let key = (self.revision, self.search.clone(), sort_code);
        if self.rows_key.as_ref() == Some(&key) {
            return;
        }
        let words: Vec<String> = search_fold(&self.search).split_whitespace().map(str::to_string).collect();
        let mut rows: Vec<(String, String, u32)> = vec![];
        for p in self.db.people.values() {
            let surname = search_fold(p.surname.trim());
            let given = search_fold(p.given.trim());
            if !words.is_empty() {
                let places: String = p.events.iter().map(|e| format!("{} {} ", e.place, e.date)).collect();
                let hay = format!("{given} {surname} {} {} {}", search_fold(&p.suffix), p.id, search_fold(&places));
                if !words.iter().all(|w| hay.contains(w.as_str())) {
                    continue;
                }
            }
            rows.push((surname, given, p.id));
        }
        match self.sort {
            SortBy::Name => rows.sort(),
            SortBy::Id => rows.sort_by_key(|r| r.2),
            SortBy::Born => rows.sort_by_key(|r| (self.db.people[&r.2].birth_year().unwrap_or(i32::MAX), r.2)),
            SortBy::Died => rows.sort_by_key(|r| (self.db.people[&r.2].death_year().unwrap_or(i32::MAX), r.2)),
        }
        self.rows = rows.into_iter().map(|r| r.2).collect();
        self.rows_key = Some(key);
    }

    pub fn refresh_source_use(&mut self) {
        if self.source_use_revision == Some(self.revision) {
            return;
        }
        self.source_use = self.db.sources.keys().map(|s| (*s, self.db.source_use_count(*s))).collect();
        self.source_use_revision = Some(self.revision);
    }

    pub fn refresh_report(&mut self) {
        let person = self.selected.unwrap_or(0);
        let kind = self.report.kind;
        let key = (kind as u8, person, self.report.generations, self.revision);
        if self.report.key != Some(key) {
            self.report.built = (person != 0 || kind == reports::Kind::Surnames)
                .then(|| reports::build(&self.db, kind, person, self.report.generations));
            self.report.key = Some(key);
        }
    }

    pub fn apply_actions(&mut self, ctx: &egui::Context) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                Action::Select(id) => self.select(id),
                Action::Open(id, tab) => {
                    self.select(id);
                    self.tab = tab;
                }
                Action::EditPerson(id) => {
                    if let Some(p) = self.db.person(id) {
                        self.person_editor = Some(PersonEditor::existing(p));
                    }
                }
                Action::NewPerson => self.person_editor = Some(PersonEditor::blank(None, Sex::Unknown, "")),
                Action::AddRelative(anchor, relation) => self.picker = Some(Picker::new(anchor, relation)),
                Action::EditFamily(id) => {
                    if let Some(f) = self.db.families.get(&id) {
                        self.family_editor = Some(FamilyEditor::new(f));
                    }
                }
                Action::RemoveChild(family, child) => {
                    self.change("Removed from the family. Undo brings them back.", |db| {
                        db.remove_child(family, child);
                        Ok(())
                    });
                }
                Action::RemovePartner(family, person) => {
                    self.change("Removed from the marriage. Undo brings them back.", |db| {
                        db.remove_partner(family, person);
                        Ok(())
                    });
                }
                Action::AskDeletePerson(id) => self.confirm = Some(Confirm::DeletePerson(id)),
                Action::EditSource(id) => self.source_editor = self.db.sources.get(&id).cloned(),
                Action::NewSource => {
                    self.source_editor = Some(Source { id: 0, ..Default::default() });
                }
                Action::AskDeleteSource(id) => self.confirm = Some(Confirm::DeleteSource(id)),
                Action::AskMerge { keep, drop } => self.confirm = Some(Confirm::Merge { keep, drop }),
                Action::IgnoreDuplicate(a, b) => {
                    self.ignored_duplicates.insert((a, b));
                }
                Action::PreferParents(person, family) => {
                    self.change("Showing those parents first.", |db| {
                        let p = db.people.get_mut(&person).ok_or("That person is gone.")?;
                        p.parent_families.retain(|f| *f != family);
                        p.parent_families.insert(0, family);
                        Ok(())
                    });
                }
                Action::FindDuplicates => self.duplicates = Some(self.db.possible_duplicates()),
                Action::PrintReport => self.print_report(),
                Action::SaveReport => self.save_report(),
                Action::Import => self.request(Pending::Import, ctx),
                Action::NewFile => self.request(Pending::NewFile, ctx),
                Action::SortPeople(by) => self.sort = by,
                Action::GoHome => {
                    if let Some(h) = self.db.home_person() {
                        self.select(h);
                        self.tab = Tab::Family;
                    }
                }
                Action::SetHome(id) => {
                    let name = self.db.name_of(id);
                    self.change(&format!("{name} is now the home person."), |db| {
                        db.people.contains_key(&id).then_some(()).ok_or("That person is gone.")?;
                        db.home = Some(id);
                        Ok(())
                    });
                }
                Action::AskRenumber => self.confirm = Some(Confirm::Renumber { how: 0 }),
            }
        }
    }

    fn print_report(&mut self) {
        let Some(report) = &self.report.built else { return };
        self.print_counter += 1;
        let path = std::env::temp_dir().join(format!(
            "family-keeper-report-{}-{}.html",
            std::process::id(),
            self.print_counter
        ));
        let opened = std::fs::write(&path, reports::to_html(report, true))
            .map_err(|e| e.to_string())
            .and_then(|()| open::that(&path).map_err(|e| e.to_string()));
        match opened {
            Ok(()) => self.status = "The report opened in your web browser, ready to print.".into(),
            Err(e) => self.message = Some(("Couldn't open the report".into(), e)),
        }
    }

    fn save_report(&mut self) {
        let Some(report) = &self.report.built else { return };
        let picked = rfd::FileDialog::new()
            .set_title("Save the report")
            .set_file_name(format!("{}.html", report.title.replace(['/', '\\', ':'], "-")))
            .add_filter("Web page", &["html"])
            .save_file();
        if let Some(path) = picked {
            let path = with_extension(path, "html");
            match fkf::write_safely(&path, reports::to_html(report, false).as_bytes()) {
                Ok(()) => self.status = "Saved the report.".into(),
                Err(e) => self.message = Some(("Couldn't save the report".into(), e)),
            }
        }
    }

    pub(crate) fn answer_confirm(&mut self, answer: dialogs::Answer, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else { return };
        use dialogs::Answer::*;
        match (confirm, answer) {
            (_, Cancel) => {}
            (Confirm::Unsaved(pending), Yes) => {
                if self.save() {
                    self.run(pending, ctx);
                }
            }
            (Confirm::Unsaved(pending), No) => self.run(pending, ctx),
            (Confirm::DeletePerson(id), Yes) => {
                self.change("Deleted. Undo brings them back.", |db| {
                    db.delete_person(id);
                    Ok(())
                });
            }
            (Confirm::DeleteSource(id), Yes) => {
                self.change("Deleted the source.", |db| {
                    db.sources.remove(&id);
                    Ok(())
                });
            }
            (Confirm::Renumber { how }, Yes) => {
                let order = self.db.renumber_order(renumber_how(how));
                let mut ids = HashMap::new();
                let done = self.change("Renumbered everyone. Undo puts the old IDs back.", |db| {
                    ids = db.renumber(&order);
                    Ok(())
                });
                if done {
                    // Follow the person on screen; IDs remembered elsewhere no longer apply.
                    self.selected = self.selected.and_then(|s| ids.get(&s).copied()).or(self.db.home_person());
                    self.history.clear();
                    self.family_choice.clear();
                    self.ignored_duplicates.clear();
                    self.duplicates = None;
                }
            }
            (Confirm::Merge { keep, drop }, Yes) => {
                if self.change("Merged. Undo separates them again.", |db| db.merge_people(keep, drop)) {
                    self.selected = Some(keep);
                    self.duplicates = Some(self.db.possible_duplicates());
                }
            }
            (_, No) => {}
        }
    }
}

/// Writes a new file and swaps it in, trying a few times: a synced folder can briefly
/// refuse to open the file being replaced.
pub fn renumber_how(how: u8) -> RenumberOrder {
    match how {
        0 => RenumberOrder::Family,
        1 => RenumberOrder::Present,
        _ => RenumberOrder::Alphabetical,
    }
}
