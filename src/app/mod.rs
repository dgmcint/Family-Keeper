//! The main window: its state, undo and redo, and the frame-by-frame loop.

pub mod colors;
mod commands;
mod files;
mod screenshot;
mod window;

pub use commands::*;
use files::*;

use crate::dialogs::{self, FamilyEditor, PersonEditor, Picker};
use crate::data::*;
use crate::reports;
use crate::screens;
use eframe::egui::{self, ViewportCommand};
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
            colors::set_name(&name);
        }
        let zoom = saved("zoom").and_then(|z| z.parse().ok()).filter(|z| (0.75..=2.0).contains(z)).unwrap_or(1.0);
        let last_file = saved("last_file").map(PathBuf::from);
        Self::start(&cc.egui_ctx, zoom, last_file, screenshot, open)
    }

    pub fn start(ctx: &egui::Context, zoom: f32, last_file: Option<PathBuf>, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        colors::apply(ctx);
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
}

impl FamilyApp {
    /// One whole frame of the program.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.handle_close(&ctx);
        self.shortcuts(&ctx);
        self.refresh_rows();
        self.update_title(&ctx);

        screens::show(self, ui);
        if let Some(answer) = dialogs::show_windows(self, &ctx) {
            self.answer_confirm(answer, &ctx);
        }
        self.apply_actions(&ctx);
        self.screenshot_tick(&ctx);
    }
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
        storage.set_string("colors", colors::name().to_string());
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
        let mut editor = dialogs::PersonEditor::blank(Some((helen, Relation::Spouse)), Sex::Male, "");
        editor.draft.given = "  Sam ".into();
        app.person_editor = Some(editor);
        dialogs::save_person(&mut app);
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
        let mut editor = dialogs::PersonEditor::existing(&original);
        editor.draft.given = "Changed".into();
        editor.draft.parent_families.clear();
        editor.draft.own_families.clear();
        app.person_editor = Some(editor);
        dialogs::save_person(&mut app);
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
