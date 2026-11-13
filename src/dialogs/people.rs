//! Editing people and marriages, and picking a relative.

use super::*;
use crate::app::{Action, FamilyApp, Relation};
use crate::data::*;
use crate::app::colors;
use crate::screens::dialog_size;
use eframe::egui::{self, RichText, vec2};

pub(crate) fn person_window(app: &mut FamilyApp, ctx: &egui::Context) {
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
            egui::Panel::bottom("person buttons").frame(egui::Frame::new().fill(colors::pal().card)).show(ui, |ui| {
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
                ui.label(RichText::new("Dates can be written freely, such as 12 MAR 1850, ABT 1850 or BEF 1900.").small().color(colors::pal().muted));
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

pub(crate) fn family_window(app: &mut FamilyApp, ctx: &egui::Context) {
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
            egui::Panel::bottom("family buttons").frame(egui::Frame::new().fill(colors::pal().card)).show(ui, |ui| {
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
                    ui.label(RichText::new("None recorded").color(colors::pal().muted));
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

pub(crate) fn picker_window(app: &mut FamilyApp, ctx: &egui::Context) {
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
                        ui.label(RichText::new("No one matches.").color(colors::pal().muted));
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
