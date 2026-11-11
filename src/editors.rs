//! The windows that open over the main screen: editing a person, a marriage or a source,
//! choosing a relative, and asking for a yes or no.

use crate::app::{Action, Confirm, FamilyApp, Relation};
use crate::model::*;
use crate::theme;
use crate::views::dialog_size;
use eframe::egui::{self, RichText, vec2};
use std::collections::BTreeMap;

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

fn field(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    ui.end_row();
}

fn citations_editor(
    ui: &mut egui::Ui,
    sources: &BTreeMap<u32, Source>,
    cites: &mut Vec<Citation>,
    actions: &mut Vec<Action>,
) {
    let mut remove = None;
    for (i, c) in cites.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                let title = |s: &Source| if s.title.trim().is_empty() { "(no title)".to_string() } else { s.title.clone() };
                let current = sources.get(&c.source).map_or("(missing source)".to_string(), title);
                egui::ComboBox::from_id_salt("source")
                    .selected_text(current)
                    .width(240.0)
                    .truncate()
                    .show_ui(ui, |ui| {
                        for s in sources.values() {
                            ui.selectable_value(&mut c.source, s.id, title(s));
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut c.page).hint_text("Page or where in the source").desired_width(170.0));
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
            ui.add(egui::TextEdit::singleline(&mut c.text).hint_text("Words from the source (optional)").desired_width(f32::INFINITY));
        });
    }
    if let Some(i) = remove {
        cites.remove(i);
    }
    ui.horizontal(|ui| {
        let first = sources.keys().next().copied();
        if ui.add_enabled(first.is_some(), egui::Button::new("Cite a source").small()).clicked() {
            if let Some(source) = first {
                cites.push(Citation { source, ..Default::default() });
            }
        }
        if ui.small_button("New source…").clicked() {
            actions.push(Action::NewSource);
        }
    });
}

fn events_editor(
    ui: &mut egui::Ui,
    sources: &BTreeMap<u32, Source>,
    events: &mut Vec<Event>,
    expanded: &mut Option<usize>,
    protected: &[&str],
    offered: &[&'static str],
    new_event: &mut &'static str,
    actions: &mut Vec<Action>,
) {
    let mut remove = None;
    for (i, e) in events.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(150.0, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_width(150.0);
                        ui.add(egui::Label::new(e.label()).truncate());
                    },
                );
                ui.add(egui::TextEdit::singleline(&mut e.date).hint_text("Date").desired_width(120.0));
                let right = 118.0;
                let place_width = (ui.available_width() - right).max(80.0);
                ui.add(egui::TextEdit::singleline(&mut e.place).hint_text("Place").desired_width(place_width));
                let open = *expanded == Some(i);
                if ui.small_button(if open { "Less" } else { "More" }).clicked() {
                    *expanded = if open { None } else { Some(i) };
                }
                if !protected.contains(&e.tag.as_str()) && ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
            if takes_details(&e.tag) || (!e.value.is_empty() && e.value != "Y") {
                ui.horizontal(|ui| {
                    ui.add_space(158.0);
                    ui.add(egui::TextEdit::singleline(&mut e.value).hint_text("Details").desired_width(f32::INFINITY));
                });
            }
            if *expanded == Some(i) {
                egui::Frame::new()
                    .fill(theme::pal().paper)
                    .corner_radius(6)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        if e.tag == "EVEN" || e.tag == "FACT" {
                            ui.add(egui::TextEdit::singleline(&mut e.kind).hint_text("What kind of event").desired_width(f32::INFINITY));
                        }
                        if is_lds_event(&e.tag) {
                            ui.add(egui::TextEdit::singleline(&mut e.temple).hint_text("Temple").desired_width(f32::INFINITY));
                        }
                        ui.add(egui::TextEdit::multiline(&mut e.note).hint_text("Note about this event").desired_rows(2).desired_width(f32::INFINITY));
                        citations_editor(ui, sources, &mut e.citations, actions);
                    });
            }
        });
    }
    if let Some(i) = remove {
        events.remove(i);
        *expanded = None;
    }
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("new event")
            .selected_text(event_label(new_event))
            .width(200.0)
            .show_ui(ui, |ui| {
                for tag in offered {
                    ui.selectable_value(new_event, *tag, event_label(tag));
                }
            });
        if ui.button("Add event").clicked() {
            events.push(Event::new(new_event));
            *expanded = Some(events.len() - 1);
        }
    });
}

fn person_window(app: &mut FamilyApp, ctx: &egui::Context) {
    let Some(editor) = app.person_editor.as_mut() else { return };
    let mut finished = None;
    let title = if editor.is_new {
        match editor.link {
            Some((_, relation)) => format!("Add {}", relation.label()),
            None => "Add a person".to_string(),
        }
    } else {
        "Edit person".to_string()
    };
    let (sources, actions) = (&app.db.sources, &mut app.actions);
    let size = dialog_size(ctx, vec2(760.0, 640.0));
    egui::Window::new(title)
        .id(egui::Id::unique("person editor"))
        .collapsible(false)
        .default_size(size)
        .max_height(size.y)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Panel::bottom("person buttons").frame(egui::Frame::new().fill(theme::pal().card)).show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Save").strong()).clicked() {
                        finished = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        finished = Some(false);
                    }
                });
            });
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let d = &mut editor.draft;
                egui::Grid::new("names").num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, |ui| {
                    field(ui, "Given names", &mut d.given);
                    field(ui, "Surname", &mut d.surname);
                    field(ui, "Suffix", &mut d.suffix);
                    ui.label("Sex");
                    ui.horizontal(|ui| {
                        for s in [Sex::Male, Sex::Female, Sex::Unknown] {
                            ui.radio_value(&mut d.sex, s, s.label());
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Events").strong());
                ui.label(RichText::new("Dates can be written freely, such as 12 MAR 1850, ABT 1850 or BEF 1900.").small().color(theme::pal().muted));
                events_editor(ui, sources, &mut d.events, &mut editor.expanded, &CORE_EVENTS, PERSON_EVENTS, &mut editor.new_event, actions);
                ui.add_space(8.0);
                ui.label(RichText::new("Notes").strong());
                ui.add(egui::TextEdit::multiline(&mut d.notes).desired_rows(4).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                ui.label(RichText::new("Sources for this person").strong());
                citations_editor(ui, sources, &mut d.citations, actions);
                ui.add_space(8.0);
                ui.label(RichText::new("Photos").strong());
                let mut remove = None;
                for (i, n) in d.extra.iter().enumerate().filter(|(_, n)| n.tag == "OBJE") {
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            let file = n.child("FILE").map_or(n.value.as_str(), |f| f.value.as_str());
                            ui.add(egui::Label::new(file).truncate());
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove {
                    d.extra.remove(i);
                }
                if ui.small_button("Add a photo…").clicked() {
                    let picked = rfd::FileDialog::new()
                        .set_title("Choose a photo")
                        .add_filter("Pictures", &["jpg", "jpeg", "png", "gif", "bmp", "tif", "tiff"])
                        .pick_file();
                    if let Some(path) = picked {
                        let mut object = Node::new("OBJE", "");
                        object.children.push(Node::new("FILE", &path.to_string_lossy()));
                        d.extra.push(object);
                    }
                }
            });
        });
    match finished {
        Some(true) => save_person(app),
        Some(false) => app.person_editor = None,
        None => {}
    }
}

pub fn save_person(app: &mut FamilyApp) {
    let Some(editor) = app.person_editor.take() else { return };
    let mut draft = editor.draft;
    draft.events.retain(|e| !e.is_blank());
    draft.given = draft.given.trim().to_string();
    draft.surname = draft.surname.trim().to_string();
    draft.suffix = draft.suffix.trim().to_string();
    if editor.is_new {
        let link_to = editor.link;
        let mut new_id = 0;
        let ok = app.change("Added a person.", |db| {
            new_id = db.add_person(draft);
            match link_to {
                Some((anchor, relation)) => link(db, relation, anchor, new_id),
                None => Ok(()),
            }
        });
        if ok && link_to.is_none() {
            app.actions.push(Action::Select(new_id));
        }
    } else {
        let id = draft.id;
        app.change("Saved the changes.", |db| {
            let p = db.people.get_mut(&id).ok_or("That person is gone.")?;
            // Family links stay as they are; only this window's fields change.
            p.given = draft.given;
            p.surname = draft.surname;
            p.suffix = draft.suffix;
            p.sex = draft.sex;
            p.events = draft.events;
            p.notes = draft.notes;
            p.citations = draft.citations;
            p.extra = draft.extra;
            Ok(())
        });
    }
}

fn family_window(app: &mut FamilyApp, ctx: &egui::Context) {
    let Some(editor) = app.family_editor.as_mut() else { return };
    let mut finished = None;
    let (db, actions) = (&app.db, &mut app.actions);
    let size = dialog_size(ctx, vec2(720.0, 560.0));
    egui::Window::new("Edit marriage")
        .id(egui::Id::unique("family editor"))
        .collapsible(false)
        .default_size(size)
        .max_height(size.y)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Panel::bottom("family buttons").frame(egui::Frame::new().fill(theme::pal().card)).show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Save").strong()).clicked() {
                        finished = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        finished = Some(false);
                    }
                });
            });
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let d = &mut editor.draft;
                let names = [d.husband, d.wife].into_iter().flatten().map(|p| db.name_of(p)).collect::<Vec<_>>().join(" and ");
                ui.label(RichText::new(names).size(17.0).strong());
                ui.add_space(4.0);
                events_editor(ui, &db.sources, &mut d.events, &mut editor.expanded, &["MARR"], FAMILY_EVENTS, &mut editor.new_event, actions);
                ui.add_space(8.0);
                ui.label(RichText::new("Children, in order").strong());
                if d.children.is_empty() {
                    ui.label(RichText::new("None recorded").color(theme::pal().muted));
                }
                let mut swap = None;
                let count = d.children.len();
                for (i, c) in d.children.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            if ui.add_enabled(i > 0, egui::Button::new("Up").small()).clicked() {
                                swap = Some((i, i - 1));
                            }
                            if ui.add_enabled(i + 1 < count, egui::Button::new("Down").small()).clicked() {
                                swap = Some((i, i + 1));
                            }
                            let life = db.person(*c).map(Person::lifespan).unwrap_or_default();
                            ui.label(format!("{}  {life}", db.name_of(*c)));
                        });
                    });
                }
                if let Some((a, b)) = swap {
                    d.children.swap(a, b);
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Notes").strong());
                ui.add(egui::TextEdit::multiline(&mut d.notes).desired_rows(3).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                ui.label(RichText::new("Sources for this marriage").strong());
                citations_editor(ui, &db.sources, &mut d.citations, actions);
            });
        });
    match finished {
        Some(true) => {
            let Some(editor) = app.family_editor.take() else { return };
            let mut draft = editor.draft;
            draft.events.retain(|e| !e.is_blank());
            let id = draft.id;
            app.change("Saved the marriage.", |db| {
                let f = db.families.get_mut(&id).ok_or("That marriage is gone.")?;
                f.events = draft.events;
                f.notes = draft.notes;
                f.citations = draft.citations;
                // Only the order can change here, and only if nobody came or went meanwhile.
                if f.children.len() == draft.children.len() {
                    f.children = draft.children;
                }
                Ok(())
            });
        }
        Some(false) => app.family_editor = None,
        None => {}
    }
}

fn picker_window(app: &mut FamilyApp, ctx: &egui::Context) {
    let Some(picker) = app.picker.as_mut() else { return };
    let mut close = false;
    let mut chosen = None;
    let mut create = false;
    let db = &app.db;
    if picker.results_for.as_deref() != Some(picker.query.as_str()) {
        let words: Vec<String> = search_fold(&picker.query).split_whitespace().map(str::to_string).collect();
        let mut found: Vec<(String, u32)> = db
            .people
            .values()
            .filter(|p| p.id != picker.anchor)
            .filter_map(|p| {
                let name = search_fold(&p.list_name());
                words.iter().all(|w| name.contains(w.as_str())).then_some((name, p.id))
            })
            .collect();
        found.sort();
        picker.results = found.into_iter().map(|f| f.1).take(300).collect();
        picker.results_for = Some(picker.query.clone());
    }
    let size = dialog_size(ctx, vec2(480.0, 480.0));
    egui::Window::new(format!("Add {} for {}", picker.relation.label(), db.name_of(picker.anchor)))
        .id(egui::Id::unique("picker"))
        .collapsible(false)
        .default_size(size)
        .max_height(size.y)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if ui.button(RichText::new("Create a new person").strong()).clicked() {
                create = true;
            }
            ui.add_space(4.0);
            ui.label("Or choose someone already in this file:");
            ui.add(egui::TextEdit::singleline(&mut picker.query).hint_text("Search names").desired_width(f32::INFINITY));
            egui::ScrollArea::vertical().auto_shrink(false).max_height(size.y - 170.0).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    for id in &picker.results {
                        let Some(p) = db.person(*id) else { continue };
                        let life = p.lifespan();
                        let text = if life.is_empty() { p.list_name() } else { format!("{}  ({life})", p.list_name()) };
                        if ui.selectable_label(false, text).clicked() {
                            chosen = Some(*id);
                        }
                    }
                    if picker.results.is_empty() {
                        ui.label(RichText::new("No one matches.").color(theme::pal().muted));
                    }
                });
            });
            ui.separator();
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    let (anchor, relation) = (picker.anchor, picker.relation);
    if close || chosen.is_some() || create {
        app.picker = None;
    }
    if let Some(other) = chosen {
        app.change("Linked them.", |db| link(db, relation, anchor, other));
    }
    if create {
        let family_name = match relation {
            Relation::Father | Relation::Sibling => app.db.person(anchor).map_or(String::new(), |p| p.surname.clone()),
            Relation::Child(family) => {
                let husband = family.and_then(|f| app.db.families.get(&f)).and_then(|f| f.husband);
                let from = husband.unwrap_or(anchor);
                app.db.person(from).map_or(String::new(), |p| p.surname.clone())
            }
            Relation::Mother | Relation::Spouse => String::new(),
        };
        let anchor_sex = app.db.person(anchor).map_or(Sex::Unknown, |p| p.sex);
        let sex = match relation {
            Relation::Father => Sex::Male,
            Relation::Mother => Sex::Female,
            Relation::Spouse => match anchor_sex {
                Sex::Male => Sex::Female,
                Sex::Female => Sex::Male,
                Sex::Unknown => Sex::Unknown,
            },
            _ => Sex::Unknown,
        };
        app.person_editor = Some(PersonEditor::blank(Some((anchor, relation)), sex, &family_name));
    }
}

fn source_window(app: &mut FamilyApp, ctx: &egui::Context) {
    let Some(draft) = app.source_editor.as_mut() else { return };
    let mut finished = None;
    egui::Window::new(if draft.id == 0 { "Add a source" } else { "Edit source" })
        .id(egui::Id::unique("source editor"))
        .collapsible(false)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("source fields").num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, |ui| {
                field(ui, "Title", &mut draft.title);
                field(ui, "Author", &mut draft.author);
                field(ui, "Publication", &mut draft.publication);
            });
            ui.label("Words from the source");
            ui.add(egui::TextEdit::multiline(&mut draft.text).desired_rows(3).desired_width(f32::INFINITY));
            ui.label("Notes");
            ui.add(egui::TextEdit::multiline(&mut draft.notes).desired_rows(3).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                let can_save = !draft.title.trim().is_empty();
                if ui.add_enabled(can_save, egui::Button::new(RichText::new("Save").strong())).on_disabled_hover_text("A source needs a title.").clicked() {
                    finished = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    finished = Some(false);
                }
            });
        });
    match finished {
        Some(true) => {
            let Some(mut source) = app.source_editor.take() else { return };
            app.change("Saved the source.", |db| {
                if source.id == 0 {
                    source.id = db.next_source_id();
                }
                db.sources.insert(source.id, source);
                Ok(())
            });
        }
        Some(false) => app.source_editor = None,
        None => {}
    }
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

fn renumber_window(app: &mut FamilyApp, ctx: &egui::Context) -> Option<Answer> {
    let Some(Confirm::Renumber { how }) = app.confirm.as_mut() else { return None };
    let db = &app.db;
    let mut answer = None;
    egui::Modal::new(egui::Id::unique("renumber")).show(ctx, |ui| {
        ui.set_max_width(480.0);
        ui.label(RichText::new("Renumber people").size(17.0).strong());
        ui.add_space(4.0);
        ui.label("Gives everyone new ID numbers 1, 2, 3... with no gaps, and numbers the marriages to match. Every link between people stays the same.");
        ui.add_space(6.0);
        for (i, label) in [
            "Family order: the home person, then parents, grandparents and the rest of the family",
            "Present order, closing the gaps",
            "Alphabetical by surname",
        ]
        .iter()
        .enumerate()
        {
            ui.radio_value(how, i as u8, *label);
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Each person keeps their old ID, shown on their card and written to exported GEDCOM files. Undo puts everything back.").color(theme::pal().muted));
        ui.add_space(6.0);
        let order = db.renumber_order(crate::app::renumber_how(*how));
        ui.label(RichText::new("First people after renumbering:").strong());
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Grid::new("renum").striped(true).num_columns(3).show(ui, |ui| {
                ui.label(RichText::new("New ID").color(theme::pal().muted));
                ui.label(RichText::new("Now").color(theme::pal().muted));
                ui.label(RichText::new("Name").color(theme::pal().muted));
                ui.end_row();
                for (i, id) in order.iter().take(100).enumerate() {
                    ui.label((i + 1).to_string());
                    ui.label(id.to_string());
                    ui.label(db.name_of(*id));
                    ui.end_row();
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Renumber").strong()).clicked() {
                answer = Some(Answer::Yes);
            }
            if ui.button("Cancel").clicked() {
                answer = Some(Answer::Cancel);
            }
        });
    });
    answer
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
