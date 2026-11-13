//! The windows that open over the main screen: shared types and the window loop.

mod people;
mod sources;
mod tools;
mod widgets;

pub use people::*;
use sources::*;
use tools::*;
use widgets::*;

use crate::app::{Confirm, FamilyApp, Relation};
use crate::data::*;
use eframe::egui::{self, RichText};

#[derive(Clone, Copy)]
pub enum Answer {
    Yes,
    No,
    Cancel,
}

const CORE_EVENTS: [&str; 4] = ["BIRT", "CHR", "DEAT", "BURI"];

/// Facts whose main content is a word rather than a date, such as an occupation.
fn takes_details(tag: &str) -> bool {
    matches!(tag, "OCCU" | "EDUC" | "RELI" | "TITL" | "NATI" | "CAST" | "DSCR" | "IDNO" | "PROP" | "SSN" | "FACT")
}

pub struct PersonEditor {
    pub draft: Person,
    pub is_new: bool,
    pub link: Option<(u32, Relation)>,
    expanded: Option<usize>,
    new_event: &'static str,
}

impl PersonEditor {
    fn with_core_events(mut draft: Person) -> Person {
        // Birth, christening, death and burial always have a row, in order, ahead of the rest.
        let mut events = vec![];
        for tag in CORE_EVENTS {
            match draft.events.iter().position(|e| e.tag == tag) {
                Some(i) => events.push(draft.events.remove(i)),
                None => events.push(Event::new(tag)),
            }
        }
        events.append(&mut draft.events);
        draft.events = events;
        draft
    }

    pub fn existing(p: &Person) -> Self {
        PersonEditor { draft: Self::with_core_events(p.clone()), is_new: false, link: None, expanded: None, new_event: "OCCU" }
    }

    pub fn blank(link: Option<(u32, Relation)>, sex: Sex, surname: &str) -> Self {
        let draft = Person { sex, surname: surname.to_string(), ..Default::default() };
        PersonEditor { draft: Self::with_core_events(draft), is_new: true, link, expanded: None, new_event: "OCCU" }
    }
}

pub struct FamilyEditor {
    pub draft: Family,
    expanded: Option<usize>,
    new_event: &'static str,
}

impl FamilyEditor {
    pub fn new(f: &Family) -> Self {
        let mut draft = f.clone();
        if draft.event("MARR").is_none() {
            draft.events.insert(0, Event::new("MARR"));
        }
        FamilyEditor { draft, expanded: None, new_event: "DIV" }
    }
}

pub struct Picker {
    pub anchor: u32,
    pub relation: Relation,
    query: String,
    results: Vec<u32>,
    results_for: Option<String>,
}

impl Picker {
    pub fn new(anchor: u32, relation: Relation) -> Self {
        Picker { anchor, relation, query: String::new(), results: vec![], results_for: None }
    }
}

pub fn link(db: &mut Database, relation: Relation, anchor: u32, other: u32) -> Result<(), String> {
    match relation {
        Relation::Father => db.add_parent(anchor, other, true),
        Relation::Mother => db.add_parent(anchor, other, false),
        Relation::Spouse => db.add_spouse(anchor, other).map(|_| ()),
        Relation::Child(family) => db.add_child(anchor, family, other).map(|_| ()),
        Relation::Sibling => db.add_sibling(anchor, other),
    }
}

/// Shows whichever windows are open. Returns the answer to a yes-or-no question, if one was given.
pub fn show_windows(app: &mut FamilyApp, ctx: &egui::Context) -> Option<Answer> {
    person_window(app, ctx);
    family_window(app, ctx);
    picker_window(app, ctx);
    source_window(app, ctx);
    message_window(app, ctx);
    confirm_window(app, ctx)
}

fn message_window(app: &mut FamilyApp, ctx: &egui::Context) {
    let Some((title, text)) = &app.message else { return };
    let mut close = false;
    egui::Modal::new(egui::Id::unique("message")).show(ctx, |ui| {
        ui.set_max_width(460.0);
        ui.label(RichText::new(title).size(17.0).strong());
        ui.add_space(4.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.label(text);
        });
        ui.add_space(6.0);
        if ui.button("OK").clicked() {
            close = true;
        }
    });
    if close {
        app.message = None;
    }
}

fn confirm_window(app: &mut FamilyApp, ctx: &egui::Context) -> Option<Answer> {
    if matches!(app.confirm, Some(Confirm::Renumber { .. })) {
        return renumber_window(app, ctx);
    }
    let confirm = app.confirm.as_ref()?;
    let db = &app.db;
    let (title, text, yes, no) = match confirm {
        Confirm::Unsaved(_) => (
            "Save your changes?".to_string(),
            "This family file has changes that haven't been saved.".to_string(),
            "Save",
            Some("Don't save"),
        ),
        Confirm::DeletePerson(id) => (
            format!("Delete {}?", db.name_of(*id)),
            "They'll be taken out of their families and their details removed. Undo brings them back until you close this file.".to_string(),
            "Delete",
            None,
        ),
        Confirm::DeleteSource(id) => (
            "Delete this source?".to_string(),
            db.sources.get(id).map_or(String::new(), |s| s.title.clone()),
            "Delete",
            None,
        ),
        Confirm::Merge { keep, drop } => (
            "Merge these two people?".to_string(),
            format!(
                "{} (ID {drop}) will be merged into {} (ID {keep}). Their events, sources, notes and families carry over. Undo separates them again until you close this file.",
                db.name_of(*drop),
                db.name_of(*keep)
            ),
            "Merge",
            None,
        ),
        Confirm::Renumber { .. } => return None,
    };
    let mut answer = None;
    egui::Modal::new(egui::Id::unique("confirm")).show(ctx, |ui| {
        ui.set_max_width(440.0);
        ui.label(RichText::new(title).size(17.0).strong());
        ui.add_space(4.0);
        ui.label(text);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(RichText::new(yes).strong()).clicked() {
                answer = Some(Answer::Yes);
            }
            if let Some(no) = no {
                if ui.button(no).clicked() {
                    answer = Some(Answer::No);
                }
            }
            if ui.button("Cancel").clicked() {
                answer = Some(Answer::Cancel);
            }
        });
    });
    answer
}
