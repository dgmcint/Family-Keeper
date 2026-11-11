//! The program itself: what's open, what's selected, and what happens when you choose something.

use crate::editors::{self, FamilyEditor, PersonEditor, Picker};
use crate::model::*;
use crate::reports;
use crate::storage;
use crate::theme;
use crate::views;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, ViewportCommand};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Family,
    Pedigree,
    People,
    Families,
    Sources,
    Duplicates,
    Reports,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 7] = [
        (Tab::Family, "Family"),
        (Tab::Pedigree, "Pedigree"),
        (Tab::People, "People"),
        (Tab::Families, "Marriages"),
        (Tab::Sources, "Sources"),
        (Tab::Duplicates, "Duplicates"),
        (Tab::Reports, "Reports"),
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relation {
    Father,
    Mother,
    Spouse,
    /// A child in this family, or in a new one when there isn't one yet.
    Child(Option<u32>),
    Sibling,
}

impl Relation {
    pub fn label(self) -> &'static str {
        match self {
            Relation::Father => "father",
            Relation::Mother => "mother",
            Relation::Spouse => "spouse",
            Relation::Child(_) => "child",
            Relation::Sibling => "brother or sister",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Id,
    Born,
    Died,
}

/// Something the person chose on screen, carried out once the screen is finished drawing.
pub enum Action {
    Select(u32),
    Open(u32, Tab),
    EditPerson(u32),
    NewPerson,
    AddRelative(u32, Relation),
    EditFamily(u32),
    RemoveChild(u32, u32),
    RemovePartner(u32, u32),
    AskDeletePerson(u32),
    EditSource(u32),
    NewSource,
    AskDeleteSource(u32),
    AskMerge { keep: u32, drop: u32 },
    IgnoreDuplicate(u32, u32),
    PreferParents(u32, u32),
    FindDuplicates,
    PrintReport,
    SaveReport,
    Import,
    NewFile,
    SortPeople(SortBy),
    GoHome,
    SetHome(u32),
    AskRenumber,
}

#[derive(Clone, Copy)]
pub enum Pending {
    NewFile,
    Open,
    Import,
    Quit,
}

pub enum Confirm {
    Unsaved(Pending),
    DeletePerson(u32),
    DeleteSource(u32),
    Merge { keep: u32, drop: u32 },
    /// Renumber people, in the order chosen (0 family, 1 present, 2 alphabetical).
    Renumber { how: u8 },
}

pub struct ReportState {
    pub kind: reports::Kind,
    pub generations: usize,
    pub built: Option<reports::Report>,
    key: Option<(u8, u32, usize, u64)>,
}

pub struct FamilyApp {
    pub db: Database,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub is_sample: bool,
    undo: Vec<Database>,
    redo: Vec<Database>,
    pub revision: u64,

    pub selected: Option<u32>,
    history: Vec<u32>,
    pub tab: Tab,
    pub search: String,
    /// The list on the left and the People table, after the search and sort.
    pub rows: Vec<u32>,
    pub sort: SortBy,
    rows_key: Option<(u64, String, u8)>,
    pub source_use: HashMap<u32, usize>,
    source_use_revision: Option<u64>,
    pub family_choice: HashMap<u32, usize>,
    pub textures: HashMap<String, Option<egui::TextureHandle>>,
    pub pedigree_generations: usize,

    pub duplicates: Option<Vec<(u32, u32)>>,
    pub ignored_duplicates: HashSet<(u32, u32)>,
    pub report: ReportState,

    pub actions: Vec<Action>,
    pub person_editor: Option<PersonEditor>,
    pub family_editor: Option<FamilyEditor>,
    pub source_editor: Option<Source>,
    pub picker: Option<Picker>,
    pub confirm: Option<Confirm>,
    pub message: Option<(String, String)>,
    pub status: String,

    allow_close: bool,
    title: String,
    pub zoom: f32,
    screenshot: Option<PathBuf>,
    frames: u32,
    screenshot_sent: bool,
    print_counter: u32,
}

impl FamilyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        let saved = |key: &str| {
            if screenshot.is_some() {
                None
            } else {
                cc.storage.and_then(|s| s.get_string(key))
            }
        };
        if let Some(name) = saved("colors") {
            theme::set_name(&name);
        }
        let zoom = saved("zoom").and_then(|z| z.parse().ok()).filter(|z| (0.75..=2.0).contains(z)).unwrap_or(1.0);
        let last_file = saved("last_file").map(PathBuf::from);
        Self::start(&cc.egui_ctx, zoom, last_file, screenshot, open)
    }

    pub fn start(ctx: &egui::Context, zoom: f32, last_file: Option<PathBuf>, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        theme::apply(ctx);
        ctx.set_zoom_factor(zoom);
        let db = Database::sample();
        let selected = db.people.keys().next().copied();
        let mut app = FamilyApp {
            db,
            path: None,
            dirty: false,
            is_sample: true,
            undo: vec![],
            redo: vec![],
            revision: 0,
            selected,
            history: vec![],
            tab: Tab::Family,
            search: String::new(),
            rows: vec![],
            sort: SortBy::Name,
            rows_key: None,
            source_use: HashMap::new(),
            source_use_revision: None,
            family_choice: HashMap::new(),
            textures: HashMap::new(),
            pedigree_generations: 4,
            duplicates: None,
            ignored_duplicates: HashSet::new(),
            report: ReportState { kind: reports::Kind::Pedigree, generations: 4, built: None, key: None },
            actions: vec![],
            person_editor: None,
            family_editor: None,
            source_editor: None,
            picker: None,
            confirm: None,
            message: None,
            status: "Ready".into(),
            allow_close: false,
            title: String::new(),
            zoom,
            screenshot,
            frames: 0,
            screenshot_sent: false,
            print_counter: 0,
        };
        if let Some(path) = open {
            app.load(&path);
        } else if let Some(path) = last_file.filter(|p| p.exists()) {
            app.load(&path);
        }
        app
    }

    /// Starts on a particular screen, used to take pictures of each one.
    pub fn show_screen(&mut self, what: &str) {
        let first = self.selected;
        match what {
            "family" => self.tab = Tab::Family,
            "pedigree" => self.tab = Tab::Pedigree,
            "people" => self.tab = Tab::People,
            "marriages" => self.tab = Tab::Families,
            "sources" => self.tab = Tab::Sources,
            "reports" => self.tab = Tab::Reports,
            "duplicates" => {
                let copy = self.db.people[&1].clone();
                self.db.add_person(Person { events: copy.events, given: copy.given, surname: copy.surname, ..Default::default() });
                self.revision += 1;
                self.tab = Tab::Duplicates;
                self.duplicates = Some(self.db.possible_duplicates());
            }
            "empty" => self.install(Database::default(), None),
            "edit-person" => self.actions.extend(first.map(Action::EditPerson)),
            "edit-marriage" => {
                let family = first.and_then(|p| self.db.people[&p].own_families.first().copied());
                self.actions.extend(family.map(Action::EditFamily));
            }
            "add-relative" => self.actions.extend(first.map(|p| Action::AddRelative(p, Relation::Child(None)))),
            "edit-source" => self.actions.extend(self.db.sources.keys().next().map(|s| Action::EditSource(*s))),
            "delete" => self.actions.extend(first.map(Action::AskDeletePerson)),
            other => eprintln!("There's no screen called {other}."),
        }
    }

    pub fn file_label(&self) -> String {
        if self.is_sample {
            return "Sample family".into();
        }
        match &self.path {
            Some(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            None => "Not saved yet".into(),
        }
    }

    /// True while a window is asking for an answer; everything behind it waits.
    pub fn busy(&self) -> bool {
        self.person_editor.is_some()
            || self.family_editor.is_some()
            || self.source_editor.is_some()
            || self.picker.is_some()
            || self.confirm.is_some()
            || self.message.is_some()
    }

    // ---------- Changes and undo ----------

    fn before_change(&mut self) {
        self.undo.push(self.db.clone());
        if self.undo.len() > 30 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn after_change(&mut self) {
        self.dirty = true;
        self.revision += 1;
        self.duplicates = None;
    }

    /// Tries a change on a copy, so a refused change leaves everything as it was.
    pub fn change(&mut self, what: &str, f: impl FnOnce(&mut Database) -> Result<(), String>) -> bool {
        let mut next = self.db.clone();
        match f(&mut next) {
            Ok(()) => {
                self.before_change();
                self.db = next;
                self.after_change();
                self.status = what.to_string();
                self.fix_selection();
                true
            }
            Err(e) => {
                self.message = Some(("That can't be done".into(), e));
                false
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.db, previous));
            self.after_change();
            self.fix_selection();
            self.status = "Undid the last change".into();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.db, next));
            self.after_change();
            self.fix_selection();
            self.status = "Redid the change".into();
        }
    }

    fn fix_selection(&mut self) {
        if !self.selected.is_some_and(|s| self.db.people.contains_key(&s)) {
            self.selected = self.db.home_person();
        }
    }

    fn select(&mut self, id: u32) {
        if self.selected != Some(id) && self.db.people.contains_key(&id) {
            if let Some(old) = self.selected {
                self.history.push(old);
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            self.selected = Some(id);
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn go_back(&mut self) {
        while let Some(id) = self.history.pop() {
            if self.db.people.contains_key(&id) {
                self.selected = Some(id);
                return;
            }
        }
    }

    // ---------- Files ----------

    fn install(&mut self, db: Database, path: Option<PathBuf>) {
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
        match storage::open(path) {
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

    fn run(&mut self, pending: Pending, ctx: &egui::Context) {
        match pending {
            Pending::NewFile => {
                self.install(Database::default(), None);
                self.status = "Started a new family file".into();
            }
            Pending::Open => {
                let picked = rfd::FileDialog::new()
                    .set_title("Open a family file")
                    .add_filter("Family files and GEDCOM", &[storage::FILE_EXTENSION, storage::OLD_EXTENSION, "ged", "GED", "gedcom"])
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
            .set_file_name(format!("Family.{}", storage::FILE_EXTENSION))
            .add_filter("Family file", &[storage::FILE_EXTENSION])
            .save_file();
        match picked {
            Some(path) => self.save_to(with_extension(path, storage::FILE_EXTENSION)),
            None => false,
        }
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        match storage::save(&path, &self.db) {
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
            match storage::write_safely(&path, crate::gedcom::export(&self.db).as_bytes()) {
                Ok(()) => self.status = format!("Exported {}", path.file_name().unwrap_or_default().to_string_lossy()),
                Err(e) => self.message = Some(("Couldn't export".into(), e)),
            }
        }
    }

    // ---------- Things kept ready for the screens ----------

    fn refresh_rows(&mut self) {
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

    // ---------- Doing what was chosen ----------

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
            match storage::write_safely(&path, reports::to_html(report, false).as_bytes()) {
                Ok(()) => self.status = "Saved the report.".into(),
                Err(e) => self.message = Some(("Couldn't save the report".into(), e)),
            }
        }
    }

    fn answer_confirm(&mut self, answer: editors::Answer, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else { return };
        use editors::Answer::*;
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

    // ---------- Keys, title, closing ----------


    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let cmd = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let shift = |key| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key);
        let pressed = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));
        if pressed(shift(Key::S)) {
            self.save_as();
        } else if pressed(cmd(Key::S)) {
            self.save();
        } else if pressed(cmd(Key::O)) {
            self.request(Pending::Open, ctx);
        } else if pressed(cmd(Key::N)) {
            self.request(Pending::NewFile, ctx);
        } else if !ctx.egui_wants_keyboard_input() {
            // While typing in the search box these keys belong to the typing.
            if pressed(shift(Key::Z)) {
                self.redo();
            } else if pressed(cmd(Key::Z)) {
                self.undo();
            }
        }
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.person_editor.is_some() || self.family_editor.is_some() || self.source_editor.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.status = "Finish or cancel the open window first.".into();
            } else if self.dirty {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                if self.confirm.is_none() {
                    self.confirm = Some(Confirm::Unsaved(Pending::Quit));
                }
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = format!("{}{} – Family Keeper", if self.dirty { "• " } else { "" }, self.file_label());
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        self.zoom = zoom;
        ctx.set_zoom_factor(zoom);
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
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

impl FamilyApp {
    /// One whole frame of the program.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.handle_close(&ctx);
        self.shortcuts(&ctx);
        self.refresh_rows();
        self.update_title(&ctx);

        views::show(self, ui);
        if let Some(answer) = editors::show_windows(self, &ctx) {
            self.answer_confirm(answer, &ctx);
        }
        self.apply_actions(&ctx);
        self.screenshot_tick(&ctx);
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

fn write_picture(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
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

fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    if path.extension().is_some() { path } else { path.with_extension(ext) }
}

impl eframe::App for FamilyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.screenshot.is_some() {
            // Draw even if something else is covering the window.
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.screenshot.is_some() {
            return;
        }
        storage.set_string("zoom", self.zoom.to_string());
        storage.set_string("colors", theme::name().to_string());
        match &self.path {
            Some(p) => storage.set_string("last_file", p.to_string_lossy().into_owned()),
            None => storage.set_string("last_file", String::new()),
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // A picture run shows the program as it starts, wherever the mouse happens to be.
        if self.screenshot.is_some() {
            raw_input.events.clear();
        }
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_app(ctx: &egui::Context) -> FamilyApp {
        FamilyApp::start(ctx, 1.0, None, None, None)
    }

    fn frames(ctx: &egui::Context, app: &mut FamilyApp, count: usize) {
        for _ in 0..count {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.draw(ui));
            output.textures_delta.clear();
        }
    }

    #[test]
    fn every_screen_draws_with_a_family_and_when_empty() {
        for what in [
            "family", "pedigree", "people", "marriages", "sources", "reports", "duplicates", "edit-person",
            "edit-marriage", "add-relative", "edit-source", "delete", "empty",
        ] {
            let ctx = egui::Context::default();
            let mut app = new_app(&ctx);
            app.show_screen(what);
            frames(&ctx, &mut app, 3);
            app.search = "alder".into();
            frames(&ctx, &mut app, 2);
            app.search = "nobody at all".into();
            frames(&ctx, &mut app, 2);
        }
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        app.show_screen("empty");
        for (tab, _) in Tab::ALL {
            app.tab = tab;
            frames(&ctx, &mut app, 2);
        }
    }

    #[test]
    fn a_new_relative_is_saved_and_linked_and_undo_takes_it_back() {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        let helen = app.db.people.values().find(|p| p.given == "Helen").unwrap().id;
        let before = app.db.clone();
        app.selected = Some(helen);
        let mut editor = editors::PersonEditor::blank(Some((helen, Relation::Spouse)), Sex::Male, "");
        editor.draft.given = "  Sam ".into();
        app.person_editor = Some(editor);
        editors::save_person(&mut app);
        let sam = app.db.people.values().find(|p| p.given == "Sam").unwrap();
        assert!(sam.events.is_empty(), "blank event rows aren't kept");
        assert_eq!(app.db.people[&helen].own_families, sam.own_families);
        assert!(app.dirty);
        app.undo();
        assert_eq!(app.db, before);
        app.redo();
        assert_eq!(app.db.people.len(), before.people.len() + 1);
    }

    #[test]
    fn editing_a_person_keeps_their_family_links_and_unknown_details() {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        let id = 1;
        app.db.people.get_mut(&id).unwrap().extra.push(Node::new("_UID", "KEEP"));
        let original = app.db.people[&id].clone();
        let mut editor = editors::PersonEditor::existing(&original);
        editor.draft.given = "Changed".into();
        editor.draft.parent_families.clear();
        editor.draft.own_families.clear();
        app.person_editor = Some(editor);
        editors::save_person(&mut app);
        let now = &app.db.people[&id];
        assert_eq!(now.given, "Changed");
        assert_eq!(now.parent_families, original.parent_families);
        assert_eq!(now.own_families, original.own_families);
        assert_eq!(now.extra, original.extra);
        assert_eq!(now.date_of("BIRT"), original.date_of("BIRT"));
    }

    #[test]
    fn a_refused_change_changes_nothing() {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        let before = app.db.clone();
        let done = app.change("x", |db| {
            db.add_person(Person::default());
            Err("no".into())
        });
        assert!(!done);
        assert_eq!(app.db, before);
        assert!(!app.dirty && !app.can_undo());
        assert!(app.message.is_some());
    }

    #[test]
    fn opening_keeps_unsaved_work_safe_by_asking_first() {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        app.change("edit", |db| {
            db.add_person(Person::default());
            Ok(())
        });
        app.request(Pending::NewFile, &ctx);
        assert!(matches!(app.confirm, Some(Confirm::Unsaved(_))));
        assert!(!app.db.people.is_empty());
    }
}
