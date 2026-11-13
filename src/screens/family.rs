//! The Family screen.

use crate::i18n::{tr, trf};
use super::*;
use crate::app::{Action, FamilyApp, Relation};
use crate::data::*;
use crate::app::colors;
use eframe::egui::{self, RichText};

pub(crate) fn family_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let Some(id) = app.selected.filter(|s| app.db.people.contains_key(s)) else {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.heading(tr("This family file is empty"));
            ui.label(RichText::new(tr("Add the first person, or import a GEDCOM file.")).color(colors::pal().muted));
            ui.add_space(8.0);
            if ui.button(tr("Add the first person")).clicked() {
                app.actions.push(Action::NewPerson);
            }
            if ui.button(tr("Import a GEDCOM file")).clicked() {
                app.actions.push(Action::Import);
            }
        });
        return;
    };
    let ctx = ui.ctx().clone();
    let (db, actions, choice, textures) = (&app.db, &mut app.actions, &mut app.family_choice, &mut app.textures);
    let person = &db.people[&id];
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // Parents
        section(ui, "PARENTS");
        ui.columns(2, |c| {
            let father = db.father_of(id);
            let mother = db.mother_of(id);
            relative_card(&mut c[0], db, actions, "FATHER", father, Some((id, Relation::Father)), 96.0);
            relative_card(&mut c[1], db, actions, "MOTHER", mother, Some((id, Relation::Mother)), 96.0);
        });
        // Adopted and step-parents are other sets of parents.
        for f in person.parent_families.iter().skip(1).filter_map(|f| db.families.get(f)) {
            ui.horizontal_wrapped(|ui| {
                let names: Vec<String> = [f.husband, f.wife].into_iter().flatten().map(|p| db.name_of(p)).collect();
                let names = if names.is_empty() { tr("Unknown").to_string() } else { names.join(tr(" and ")) };
                ui.label(RichText::new(trf("Also has parents: {names}", &[("names", &names)])).color(colors::pal().muted));
                if ui.small_button(tr("Show these first")).clicked() {
                    actions.push(Action::PreferParents(id, f.id));
                }
            });
        }
        ui.add_space(10.0);

        // This person and their spouse
        let families: Vec<&Family> = person.own_families.iter().filter_map(|f| db.families.get(f)).collect();
        let index = choice.get(&id).copied().unwrap_or(0).min(families.len().saturating_sub(1));
        let family = families.get(index).copied();
        section(ui, "THIS PERSON AND SPOUSE");
        ui.columns(2, |c| {
            card_frame(true).show(&mut c[0], |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(150.0);
                ui.horizontal(|ui| {
                    let photo = db.photos_of(person).into_iter().find_map(|path| photo_texture(&ctx, textures, &path));
                    if let Some(tex) = photo {
                        let size = tex.size_vec2();
                        let scale = (96.0 / size.y).min(96.0 / size.x);
                        ui.add(egui::Image::new((tex.id(), size * scale)).corner_radius(4));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(person.name()).size(21.0).strong());
                        let mut line = format!("{}  ·  ID {}", tr(person.sex.label()), person.id);
                        if let Some(old) = Database::old_id(person) {
                            line.push_str(&trf(" (old ID {old})", &[("old", &old)]));
                        }
                        if db.home_person() == Some(id) {
                            line.push_str(&format!("  ·  {}", tr("Home person")));
                        }
                        ui.label(RichText::new(line).small().color(colors::pal().muted));
                        person_facts(ui, person);
                        ui.horizontal(|ui| {
                            if ui.button(tr("Edit")).clicked() {
                                actions.push(Action::EditPerson(id));
                            }
                            if ui.button(tr("Delete…")).clicked() {
                                actions.push(Action::AskDeletePerson(id));
                            }
                        });
                    });
                });
                if !person.notes.trim().is_empty() {
                    ui.add_space(4.0);
                    let first = person.notes.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                    ui.label(RichText::new(trf("Note: {note}", &[("note", &first)])).italics().color(colors::pal().muted));
                }
            });
            let spouse = family.and_then(|f| f.partner_of(id));
            relative_card(&mut c[1], db, actions, "SPOUSE", spouse, Some((id, Relation::Spouse)), 150.0);
        });
        ui.add_space(4.0);

        // The marriage
        ui.horizontal_wrapped(|ui| {
            if let Some(f) = family {
                let text = f.event("MARR").map(Event::summary).filter(|s| !s.is_empty());
                match text {
                    Some(t) => {
                        ui.label(RichText::new(tr("Married")).color(colors::pal().muted));
                        ui.label(t);
                    }
                    None => {
                        ui.label(RichText::new(tr("No marriage details")).color(colors::pal().muted));
                    }
                }
                if ui.small_button(tr("Edit marriage")).clicked() {
                    actions.push(Action::EditFamily(f.id));
                }
                if let Some(s) = f.partner_of(id) {
                    if ui.small_button(tr("Remove spouse")).on_hover_text(tr("Separates them. Nobody gets deleted.")).clicked() {
                        actions.push(Action::RemovePartner(f.id, s));
                    }
                }
            }
            if families.len() > 1 {
                let mut chosen = index;
                egui::ComboBox::from_id_salt("which family")
                    .selected_text(trf("Marriage {at} of {count}", &[("at", &(index + 1)), ("count", &families.len())]))
                    .show_ui(ui, |ui| {
                        for (i, f) in families.iter().enumerate() {
                            let name = f.partner_of(id).map_or(tr("Unknown spouse").to_string(), |s| db.name_of(s));
                            ui.selectable_value(&mut chosen, i, format!("{}. {name}", i + 1));
                        }
                    });
                if chosen != index {
                    choice.insert(id, chosen);
                }
            }
            if family.is_some_and(|f| f.partner_of(id).is_some()) && ui.small_button(tr("Add another spouse")).clicked() {
                actions.push(Action::AddRelative(id, Relation::Spouse));
            }
        });
        ui.add_space(10.0);

        // Children and siblings
        ui.columns(2, |c| {
            let family_id = family.map(|f| f.id);
            let kids: &[u32] = family.map_or(&[], |f| &f.children);
            kin_list(&mut c[0], db, actions, "CHILDREN", kids, family_id, (id, Relation::Child(family_id)));
            let parents = person.parent_families.first().and_then(|f| db.families.get(f));
            let sibs: Vec<u32> = parents.map_or(vec![], |f| f.children.iter().copied().filter(|c| *c != id).collect());
            kin_list(&mut c[1], db, actions, "BROTHERS AND SISTERS", &sibs, parents.map(|f| f.id), (id, Relation::Sibling));
        });
    });
}

fn kin_list(
    ui: &mut egui::Ui,
    db: &Database,
    actions: &mut Vec<Action>,
    title: &str,
    ids: &[u32],
    family: Option<u32>,
    add: (u32, Relation),
) {
    card_frame(false).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(120.0);
        section(ui, title);
        if ids.is_empty() {
            ui.label(RichText::new(tr("None recorded")).color(colors::pal().muted));
        }
        for (i, kid) in ids.iter().enumerate() {
            let Some(p) = db.person(*kid) else { continue };
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", i + 1)).color(colors::pal().muted));
                if ui.link(RichText::new(p.name()).color(colors::pal().accent)).clicked() {
                    actions.push(Action::Select(p.id));
                }
                ui.label(RichText::new(p.lifespan()).color(colors::pal().muted));
                if let Some(f) = family {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(tr("Remove")).on_hover_text(tr("Takes them out of this family. Nobody gets deleted.")).clicked() {
                            actions.push(Action::RemoveChild(f, *kid));
                        }
                    });
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.small_button(tr(add.1.add_label())).clicked() {
                actions.push(Action::AddRelative(add.0, add.1));
            }
        });
    });
}
