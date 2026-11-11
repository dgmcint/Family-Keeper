//! The main window: menus, tabs, the list of people, and each tab's screen.

use crate::app::{Action, FamilyApp, Pending, Relation, SortBy, Tab};
use crate::model::*;
use crate::reports;
use crate::theme;
use eframe::egui::{self, CornerRadius, Margin, RichText, Sense, Stroke, StrokeKind, Vec2, vec2};
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;

pub fn show(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let busy = app.busy();
    egui::Panel::top("menu")
        .frame(egui::Frame::new().fill(theme::pal().card).inner_margin(Margin::symmetric(8, 3)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| menu_bar(app, ui));
        });
    egui::Panel::top("tabs")
        .frame(egui::Frame::new().fill(theme::pal().paper).inner_margin(Margin::symmetric(12, 8)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| tabs_row(app, ui));
        });
    egui::Panel::bottom("status")
        .frame(egui::Frame::new().fill(theme::pal().card).inner_margin(Margin::symmetric(12, 4)))
        .show(ui, |ui| status_bar(app, ui));
    egui::Panel::left("people")
        .resizable(true)
        .default_size(270.0)
        .size_range(200.0..=460.0)
        .frame(egui::Frame::new().fill(theme::pal().card).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| people_list(app, ui));
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme::pal().paper).inner_margin(Margin::same(14)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                if app.is_sample {
                    sample_banner(app, ui);
                }
                match app.tab {
                    Tab::Family => family_tab(app, ui),
                    Tab::Pedigree => pedigree_tab(app, ui),
                    Tab::People => people_tab(app, ui),
                    Tab::Families => families_tab(app, ui),
                    Tab::Sources => sources_tab(app, ui),
                    Tab::Duplicates => duplicates_tab(app, ui),
                    Tab::Reports => reports_tab(app, ui),
                }
            });
        });
}

fn menu_bar(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New family file").clicked() {
                app.request(Pending::NewFile, &ctx);
                ui.close();
            }
            if ui.button("Open…").clicked() {
                app.request(Pending::Open, &ctx);
                ui.close();
            }
            ui.separator();
            if ui.button("Save").clicked() {
                app.save();
                ui.close();
            }
            if ui.button("Save as…").clicked() {
                app.save_as();
                ui.close();
            }
            ui.separator();
            if ui.button("Import GEDCOM…").clicked() {
                app.request(Pending::Import, &ctx);
                ui.close();
            }
            if ui.button("Export GEDCOM…").clicked() {
                app.export_gedcom();
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close();
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui.add_enabled(app.can_undo(), egui::Button::new("Undo")).clicked() {
                app.undo();
                ui.close();
            }
            if ui.add_enabled(app.can_redo(), egui::Button::new("Redo")).clicked() {
                app.redo();
                ui.close();
            }
            ui.separator();
            if ui.button("Add a person…").clicked() {
                app.actions.push(Action::NewPerson);
                ui.close();
            }
            let selected = app.selected;
            if ui.add_enabled(selected.is_some(), egui::Button::new("Edit this person…")).clicked() {
                app.actions.extend(selected.map(Action::EditPerson));
                ui.close();
            }
            if ui.add_enabled(selected.is_some(), egui::Button::new("Delete this person…")).clicked() {
                app.actions.extend(selected.map(Action::AskDeletePerson));
                ui.close();
            }
            ui.separator();
            let is_home = selected.is_some() && selected == app.db.home_person();
            if ui.add_enabled(selected.is_some() && !is_home, egui::Button::new("Make this person the home person")).clicked() {
                app.actions.extend(selected.map(Action::SetHome));
                ui.close();
            }
            if ui.add_enabled(!app.db.people.is_empty(), egui::Button::new("Renumber people…")).clicked() {
                app.actions.push(Action::AskRenumber);
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            for (tab, label) in Tab::ALL {
                if ui.selectable_label(app.tab == tab, label).clicked() {
                    app.tab = tab;
                    ui.close();
                }
            }
            ui.separator();
            ui.label(RichText::new("Text size").color(theme::pal().muted));
            for (zoom, label) in [(1.0, "Normal"), (1.25, "Large"), (1.5, "Extra large")] {
                if ui.selectable_label(app.zoom == zoom, label).clicked() {
                    app.set_zoom(&ctx, zoom);
                    ui.close();
                }
            }
            ui.separator();
            ui.menu_button("Colors", |ui| {
                for (i, (name, group, _)) in theme::SCHEMES.iter().enumerate() {
                    if i == 0 || theme::SCHEMES[i - 1].1 != *group {
                        if i > 0 {
                            ui.separator();
                        }
                        ui.label(RichText::new(*group).color(theme::pal().muted));
                    }
                    if ui.selectable_label(theme::current() == i, *name).clicked() {
                        theme::choose(&ctx, i);
                        ui.close();
                    }
                }
            });
        });
        ui.menu_button("Help", |ui| {
            if ui.button("About Family Keeper").clicked() {
                app.message = Some((
                    "About Family Keeper".into(),
                    format!(
                        "Version {}\n\nKeep your family history: people, marriages, events, sources, notes and photos. Share it with other family history programs using GEDCOM files.",
                        env!("CARGO_PKG_VERSION")
                    ),
                ));
                ui.close();
            }
        });
    });
}

fn tabs_row(app: &mut FamilyApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.add_enabled(app.can_go_back(), egui::Button::new("Back")).clicked() {
            app.go_back();
        }
        let home = app.db.home_person();
        let tip = home.map(|h| format!("Go to the home person, {}", app.db.name_of(h))).unwrap_or_default();
        if ui.add_enabled(home.is_some(), egui::Button::new("Home")).on_hover_text(tip).clicked() {
            app.actions.push(Action::GoHome);
        }
        ui.add_space(10.0);
        for (tab, label) in Tab::ALL {
            ui.selectable_value(&mut app.tab, tab, label);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Add a person").clicked() {
                app.actions.push(Action::NewPerson);
            }
        });
    });
}

fn status_bar(app: &FamilyApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(&app.status).color(theme::pal().muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let saved = if app.dirty { "Unsaved changes" } else { "All changes saved" };
            let saved = if app.is_sample && !app.dirty { "Sample, not saved" } else { saved };
            ui.label(RichText::new(saved).color(if app.dirty { theme::pal().gold } else { theme::pal().muted }));
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}  ·  {}  ·",
                    count(app.db.people.len(), "person", "people"),
                    count(app.db.families.len(), "marriage", "marriages"),
                    count(app.db.sources.len(), "source", "sources"),
                ))
                .color(theme::pal().muted),
            );
        });
    });
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn sample_banner(app: &mut FamilyApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(theme::pal().gold_soft)
        .stroke(Stroke::new(1.0, theme::pal().border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("This is a made-up sample family, here to show how things work.");
                if ui.button("Import my GEDCOM file").clicked() {
                    app.actions.push(Action::Import);
                }
                if ui.button("Start a new file").clicked() {
                    app.actions.push(Action::NewFile);
                }
            });
        });
    ui.add_space(10.0);
}

fn people_list(app: &mut FamilyApp, ui: &mut egui::Ui) {
    ui.add(
        egui::TextEdit::singleline(&mut app.search)
            .hint_text("Search names, places, years")
            .desired_width(f32::INFINITY),
    );
    let shown = app.rows.len();
    let total = app.db.people.len();
    let label = if shown == total { count(total, "person", "people") } else { format!("{shown} of {}", count(total, "person", "people")) };
    ui.label(RichText::new(label).small().color(theme::pal().muted));
    ui.separator();
    if app.rows.is_empty() {
        let text = if total == 0 { "Nobody here yet." } else { "No one matches that search." };
        ui.label(RichText::new(text).color(theme::pal().muted));
        return;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().button_padding.y * 2.0;
    let selected = app.selected;
    let (db, rows, actions) = (&app.db, &app.rows, &mut app.actions);
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, rows.len(), |ui, range| {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for id in &rows[range] {
                let Some(p) = db.person(*id) else { continue };
                let label = ui.selectable_label(selected == Some(*id), p.list_name());
                let label = if p.lifespan().is_empty() { label } else { label.on_hover_text(p.lifespan()) };
                if label.clicked() {
                    actions.push(Action::Select(*id));
                }
            }
        });
    });
}

// ---------- Family tab ----------

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().strong().color(theme::pal().muted));
}

fn card_frame(highlight: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(if highlight { theme::pal().accent_soft } else { theme::pal().card })
        .stroke(Stroke::new(if highlight { 1.5 } else { 1.0 }, if highlight { theme::pal().accent } else { theme::pal().border }))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
}

fn fact_line(ui: &mut egui::Ui, label: &str, event: Option<&Event>) {
    let Some(e) = event.filter(|e| !e.summary().is_empty()) else { return };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().interact_size.y = 16.0;
        ui.label(RichText::new(label).color(theme::pal().muted));
        ui.label(e.summary());
    });
}

fn person_facts(ui: &mut egui::Ui, p: &Person) {
    fact_line(ui, "Born", p.event("BIRT"));
    if p.event("BIRT").is_none_or(|e| e.summary().is_empty()) {
        fact_line(ui, "Christened", p.event("CHR"));
    }
    fact_line(ui, "Died", p.event("DEAT"));
    if p.event("DEAT").is_none_or(|e| e.summary().is_empty()) {
        fact_line(ui, "Buried", p.event("BURI"));
    }
}

fn relative_card(
    ui: &mut egui::Ui,
    db: &Database,
    actions: &mut Vec<Action>,
    label: &str,
    person: Option<u32>,
    add: Option<(u32, Relation)>,
    height: f32,
) {
    card_frame(false).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height);
        section(ui, label);
        match person.and_then(|id| db.person(id)) {
            Some(p) => {
                if ui.link(RichText::new(p.name()).size(17.0).color(theme::pal().accent)).clicked() {
                    actions.push(Action::Select(p.id));
                }
                person_facts(ui, p);
            }
            None => {
                ui.label(RichText::new("Not recorded").color(theme::pal().muted));
                if let Some((anchor, relation)) = add {
                    ui.horizontal(|ui| {
                        if ui.button(format!("Add {}", relation.label())).clicked() {
                            actions.push(Action::AddRelative(anchor, relation));
                        }
                    });
                }
            }
        }
    });
}

pub fn photo_texture(
    ctx: &egui::Context,
    cache: &mut HashMap<String, Option<egui::TextureHandle>>,
    path: &str,
) -> Option<egui::TextureHandle> {
    cache
        .entry(path.to_string())
        .or_insert_with(|| {
            let small = std::fs::metadata(path).is_ok_and(|m| m.len() < 64 * 1024 * 1024);
            if !small {
                return None;
            }
            let picture = image::open(path).ok()?.thumbnail(320, 320).to_rgba8();
            let size = [picture.width() as usize, picture.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, picture.as_raw());
            Some(ctx.load_texture(path, color, egui::TextureOptions::LINEAR))
        })
        .clone()
}

fn family_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let Some(id) = app.selected.filter(|s| app.db.people.contains_key(s)) else {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.heading("This family file is empty");
            ui.label(RichText::new("Add the first person, or import a GEDCOM file.").color(theme::pal().muted));
            ui.add_space(8.0);
            if ui.button("Add the first person").clicked() {
                app.actions.push(Action::NewPerson);
            }
            if ui.button("Import a GEDCOM file").clicked() {
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
                let names = if names.is_empty() { "Unknown".to_string() } else { names.join(" and ") };
                ui.label(RichText::new(format!("Also has parents: {names}")).color(theme::pal().muted));
                if ui.small_button("Show these first").clicked() {
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
                        let mut line = format!("{}  ·  ID {}", person.sex.label(), person.id);
                        if let Some(old) = Database::old_id(person) {
                            line.push_str(&format!(" (old ID {old})"));
                        }
                        if db.home_person() == Some(id) {
                            line.push_str("  ·  Home person");
                        }
                        ui.label(RichText::new(line).small().color(theme::pal().muted));
                        person_facts(ui, person);
                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                actions.push(Action::EditPerson(id));
                            }
                            if ui.button("Delete…").clicked() {
                                actions.push(Action::AskDeletePerson(id));
                            }
                        });
                    });
                });
                if !person.notes.trim().is_empty() {
                    ui.add_space(4.0);
                    let first = person.notes.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                    ui.label(RichText::new(format!("Note: {first}")).italics().color(theme::pal().muted));
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
                        ui.label(RichText::new("Married").color(theme::pal().muted));
                        ui.label(t);
                    }
                    None => {
                        ui.label(RichText::new("No marriage details").color(theme::pal().muted));
                    }
                }
                if ui.small_button("Edit marriage").clicked() {
                    actions.push(Action::EditFamily(f.id));
                }
                if let Some(s) = f.partner_of(id) {
                    if ui.small_button("Remove spouse").on_hover_text("Separates them. Nobody gets deleted.").clicked() {
                        actions.push(Action::RemovePartner(f.id, s));
                    }
                }
            }
            if families.len() > 1 {
                let mut chosen = index;
                egui::ComboBox::from_id_salt("which family")
                    .selected_text(format!("Marriage {} of {}", index + 1, families.len()))
                    .show_ui(ui, |ui| {
                        for (i, f) in families.iter().enumerate() {
                            let name = f.partner_of(id).map_or("Unknown spouse".to_string(), |s| db.name_of(s));
                            ui.selectable_value(&mut chosen, i, format!("{}. {name}", i + 1));
                        }
                    });
                if chosen != index {
                    choice.insert(id, chosen);
                }
            }
            if family.is_some_and(|f| f.partner_of(id).is_some()) && ui.small_button("Add another spouse").clicked() {
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
            ui.label(RichText::new("None recorded").color(theme::pal().muted));
        }
        for (i, kid) in ids.iter().enumerate() {
            let Some(p) = db.person(*kid) else { continue };
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", i + 1)).color(theme::pal().muted));
                if ui.link(RichText::new(p.name()).color(theme::pal().accent)).clicked() {
                    actions.push(Action::Select(p.id));
                }
                ui.label(RichText::new(p.lifespan()).color(theme::pal().muted));
                if let Some(f) = family {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Remove").on_hover_text("Takes them out of this family. Nobody gets deleted.").clicked() {
                            actions.push(Action::RemoveChild(f, *kid));
                        }
                    });
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.small_button(format!("Add {}", add.1.label())).clicked() {
                actions.push(Action::AddRelative(add.0, add.1));
            }
        });
    });
}

// ---------- Pedigree tab ----------

fn shorten(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut s: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    s.push('…');
    s
}

fn pedigree_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let Some(root) = app.selected else {
        ui.label(RichText::new("Choose a person to see their ancestors.").color(theme::pal().muted));
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Ancestors of {}", app.db.name_of(root))).size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for g in [5, 4, 3] {
                ui.selectable_value(&mut app.pedigree_generations, g, format!("{g}"));
            }
            ui.label(RichText::new("Generations").color(theme::pal().muted));
        });
    });
    ui.label(RichText::new("Click someone to move them to the front. Double-click to see their family.").small().color(theme::pal().muted));
    ui.add_space(4.0);

    let gens = app.pedigree_generations;
    let (db, actions) = (&app.db, &mut app.actions);
    // Everyone's place: 1 is this person, 2 and 3 their parents, 4 to 7 their grandparents...
    let mut slots: Vec<Option<u32>> = vec![None; 1 << gens];
    slots[1] = Some(root);
    for n in 1..(1usize << (gens - 1)) {
        if let Some(p) = slots[n] {
            slots[n * 2] = db.father_of(p);
            slots[n * 2 + 1] = db.mother_of(p);
        }
    }

    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let available = ui.available_size();
        let last = 1usize << (gens - 1);
        let box_h = 52.0;
        let gap_x = 30.0;
        let box_w = ((available.x - gap_x * (gens as f32 - 1.0)) / gens as f32).clamp(170.0, 300.0);
        let height = (last as f32 * (box_h + 6.0)).max(available.y - 8.0);
        let width = gens as f32 * box_w + (gens as f32 - 1.0) * gap_x;
        let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let painter = ui.painter_at(area);
        let rect_of = |n: usize| {
            let g = n.ilog2() as usize;
            let in_gen = n - (1 << g);
            let y = area.top() + (in_gen as f32 + 0.5) * height / (1usize << g) as f32;
            let x = area.left() + g as f32 * (box_w + gap_x);
            egui::Rect::from_min_size(egui::pos2(x, y - box_h / 2.0), vec2(box_w, box_h))
        };
        let line = Stroke::new(1.5, theme::pal().border);
        for n in 2..(1usize << gens) {
            let (a, b) = (rect_of(n / 2), rect_of(n));
            let mid = a.right() + gap_x / 2.0;
            painter.line_segment([a.right_center(), egui::pos2(mid, a.center().y)], line);
            painter.line_segment([egui::pos2(mid, a.center().y), egui::pos2(mid, b.center().y)], line);
            painter.line_segment([egui::pos2(mid, b.center().y), b.left_center()], line);
        }
        for n in 1..(1usize << gens) {
            let rect = rect_of(n);
            let chars = ((box_w - 20.0) / 7.2) as usize;
            let Some(p) = slots[n].and_then(|id| db.person(id)) else {
                painter.rect(rect, 6.0, theme::pal().paper, Stroke::new(1.0, theme::pal().border), StrokeKind::Inside);
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Not recorded", egui::FontId::proportional(13.0), theme::pal().muted);
                continue;
            };
            let response = ui.interact(rect, ui.scope_id().with(("pedigree", n)), Sense::click());
            let is_root = n == 1;
            let fill = if is_root {
                theme::pal().accent_soft
            } else if response.hovered() {
                theme::pal().card_hover
            } else {
                theme::pal().card
            };
            let edge = if is_root || response.hovered() { theme::pal().accent } else { theme::pal().border };
            painter.rect(rect, 6.0, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
            let left = rect.left() + 10.0;
            painter.text(
                egui::pos2(left, rect.top() + 16.0),
                egui::Align2::LEFT_CENTER,
                shorten(&p.name(), chars),
                egui::FontId::proportional(14.5),
                theme::pal().ink,
            );
            let birth_place = p.place_of("BIRT");
            let life = p.lifespan();
            let detail = match (life.is_empty(), birth_place.is_empty()) {
                (false, false) => format!("{life}, {birth_place}"),
                (false, true) => life,
                (true, false) => birth_place.to_string(),
                (true, true) => String::new(),
            };
            painter.text(
                egui::pos2(left, rect.top() + 36.0),
                egui::Align2::LEFT_CENTER,
                shorten(&detail, chars + 4),
                egui::FontId::proportional(12.5),
                theme::pal().muted,
            );
            if response.double_clicked() {
                actions.push(Action::Open(p.id, Tab::Family));
            } else if response.clicked() {
                actions.push(Action::Select(p.id));
            }
        }
    });
}

// ---------- Lists ----------

fn table_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).strong());
}

fn people_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, rows, actions, selected, sort) = (&app.db, &app.rows, &mut app.actions, app.selected, app.sort);
    if rows.is_empty() {
        ui.label(RichText::new("Nobody to show.").color(theme::pal().muted));
        return;
    }
    ui.style_mut().interaction.selectable_labels = false;
    let clicked_header = std::cell::Cell::new(None);
    TableBuilder::new(ui)
        .id_salt("people table")
        .striped(true)
        .sense(Sense::click())
        .auto_shrink(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(60.0))
        .column(Column::initial(240.0).at_least(120.0).clip(true))
        .column(Column::exact(70.0))
        .column(Column::initial(110.0).at_least(70.0).clip(true))
        .column(Column::initial(210.0).at_least(80.0).clip(true))
        .column(Column::remainder().at_least(70.0).clip(true))
        .header(28.0, |mut h| {
            for (label, by) in [
                ("ID", Some(SortBy::Id)),
                ("Name", Some(SortBy::Name)),
                ("Sex", None),
                ("Born", Some(SortBy::Born)),
                ("Birth place", None),
                ("Died", Some(SortBy::Died)),
            ] {
                h.col(|ui| match by {
                    Some(by) => {
                        ui.spacing_mut().button_padding.x = 2.0;
                        let text = if sort == by { format!("{label} ▾") } else { label.to_string() };
                        if ui.selectable_label(sort == by, RichText::new(text).strong()).clicked() {
                            clicked_header.set(Some(by));
                        }
                    }
                    None => table_heading(ui, label),
                });
            }
        })
        .body(|body| {
            body.rows(26.0, rows.len(), |mut row| {
                let id = rows[row.index()];
                let Some(p) = db.person(id) else { return };
                row.set_selected(selected == Some(id));
                row.col(|ui| {
                    ui.label(id.to_string());
                });
                row.col(|ui| {
                    ui.label(p.list_name());
                });
                row.col(|ui| {
                    ui.label(p.sex.label());
                });
                row.col(|ui| {
                    ui.label(p.date_of("BIRT"));
                });
                row.col(|ui| {
                    ui.label(p.place_of("BIRT"));
                });
                row.col(|ui| {
                    ui.label(p.date_of("DEAT"));
                });
                let response = row.response();
                if response.double_clicked() {
                    actions.push(Action::Open(id, Tab::Family));
                } else if response.clicked() {
                    actions.push(Action::Select(id));
                }
            });
        });
    if let Some(by) = clicked_header.get() {
        actions.push(Action::SortPeople(by));
    }
}

fn families_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, actions) = (&app.db, &mut app.actions);
    if db.families.is_empty() {
        ui.label(RichText::new("No marriages or families recorded yet.").color(theme::pal().muted));
        return;
    }
    let families: Vec<&Family> = db.families.values().collect();
    ui.style_mut().interaction.selectable_labels = false;
    TableBuilder::new(ui)
        .id_salt("families table")
        .striped(true)
        .sense(Sense::click())
        .auto_shrink(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(60.0))
        .column(Column::initial(220.0).at_least(120.0).clip(true))
        .column(Column::initial(220.0).at_least(120.0).clip(true))
        .column(Column::initial(110.0).at_least(70.0).clip(true))
        .column(Column::initial(180.0).at_least(80.0).clip(true))
        .column(Column::remainder().at_least(60.0))
        .header(28.0, |mut h| {
            for label in ["ID", "Husband", "Wife", "Married", "Place", "Children"] {
                h.col(|ui| table_heading(ui, label));
            }
        })
        .body(|body| {
            body.rows(26.0, families.len(), |mut row| {
                let f = families[row.index()];
                let name = |id: Option<u32>| id.and_then(|i| db.person(i)).map_or(String::new(), Person::list_name);
                let marriage = f.event("MARR");
                row.col(|ui| {
                    ui.label(f.id.to_string());
                });
                row.col(|ui| {
                    ui.label(name(f.husband));
                });
                row.col(|ui| {
                    ui.label(name(f.wife));
                });
                row.col(|ui| {
                    ui.label(marriage.map_or("", |e| e.date.as_str()));
                });
                row.col(|ui| {
                    ui.label(marriage.map_or("", |e| e.place.as_str()));
                });
                row.col(|ui| {
                    ui.label(f.children.len().to_string());
                });
                if row.response().clicked() {
                    let who = f.husband.or(f.wife).or(f.children.first().copied());
                    actions.extend(who.map(|p| Action::Open(p, Tab::Family)));
                }
            });
        });
}

fn sources_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    app.refresh_source_use();
    let (db, actions, uses) = (&app.db, &mut app.actions, &app.source_use);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Sources").size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Add a source").clicked() {
                actions.push(Action::NewSource);
            }
        });
    });
    ui.add_space(4.0);
    if db.sources.is_empty() {
        ui.label(RichText::new("No sources yet. Sources record where your information came from.").color(theme::pal().muted));
        return;
    }
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for s in db.sources.values() {
            card_frame(false).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let title = if s.title.trim().is_empty() { "(no title)" } else { s.title.as_str() };
                        ui.label(RichText::new(title).strong());
                        if !s.author.is_empty() {
                            ui.label(RichText::new(&s.author).color(theme::pal().muted));
                        }
                        let n = uses.get(&s.id).copied().unwrap_or(0);
                        ui.label(RichText::new(format!("Used {n} time{}", if n == 1 { "" } else { "s" })).small().color(theme::pal().muted));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let used = uses.get(&s.id).copied().unwrap_or(0) > 0;
                        let delete = ui.add_enabled(!used, egui::Button::new("Delete…"));
                        if delete.on_disabled_hover_text("People or events still cite this source.").clicked() {
                            actions.push(Action::AskDeleteSource(s.id));
                        }
                        if ui.button("Edit").clicked() {
                            actions.push(Action::EditSource(s.id));
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn duplicates_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, actions) = (&app.db, &mut app.actions);
    ui.label(RichText::new("Possible duplicates").size(18.0).strong());
    ui.label(RichText::new("Finds people with the same name whose birth years don't rule out a match. You choose who to merge.").color(theme::pal().muted));
    ui.add_space(4.0);
    if ui.button("Check for duplicates").clicked() {
        actions.push(Action::FindDuplicates);
    }
    ui.add_space(6.0);
    let Some(pairs) = &app.duplicates else { return };
    let ignored = &app.ignored_duplicates;
    let pairs: Vec<&(u32, u32)> = pairs.iter().filter(|p| !ignored.contains(p)).collect();
    if pairs.is_empty() {
        ui.label("No possible duplicates found.");
        return;
    }
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for &&(a, b) in &pairs {
            let (Some(pa), Some(pb)) = (db.person(a), db.person(b)) else { continue };
            card_frame(false).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.push_id((a, b), |ui| {
                    ui.columns(2, |c| {
                        for (col, p, other) in [(0, pa, pb), (1, pb, pa)] {
                            let ui = &mut c[col];
                            ui.label(RichText::new(p.name()).strong());
                            ui.label(RichText::new(format!("ID {}", p.id)).small().color(theme::pal().muted));
                            person_facts(ui, p);
                            if ui.button("Keep this one and merge").clicked() {
                                actions.push(Action::AskMerge { keep: p.id, drop: other.id });
                            }
                        }
                    });
                    if ui.small_button("These are different people").clicked() {
                        actions.push(Action::IgnoreDuplicate(a, b));
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn reports_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("report kind")
            .selected_text(app.report.kind.label())
            .width(210.0)
            .show_ui(ui, |ui| {
                for kind in reports::Kind::ALL {
                    ui.selectable_value(&mut app.report.kind, kind, kind.label());
                }
            });
        if app.report.kind.uses_generations() {
            ui.add(egui::DragValue::new(&mut app.report.generations).range(1..=12).suffix(" generations"));
        }
        if ui.button("Print…").on_hover_text("Opens in your web browser, where you can print or save a PDF.").clicked() {
            app.actions.push(Action::PrintReport);
        }
        if ui.button("Save as web page…").clicked() {
            app.actions.push(Action::SaveReport);
        }
    });
    ui.add_space(6.0);
    app.refresh_report();
    let Some(report) = &app.report.built else {
        ui.label(RichText::new("Choose a person first.").color(theme::pal().muted));
        return;
    };
    egui::Frame::new()
        .fill(theme::pal().card)
        .stroke(Stroke::new(1.0, theme::pal().border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.label(RichText::new(&report.title).size(20.0).strong());
                ui.separator();
                for block in &report.blocks {
                    match block {
                        reports::Block::Heading(s) => {
                            ui.add_space(8.0);
                            ui.label(RichText::new(s).strong().size(16.0));
                        }
                        reports::Block::Line { indent, bold, text } => {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(*indent as f32 * 22.0);
                                let t = RichText::new(text);
                                ui.label(if *bold { t.strong() } else { t });
                            });
                        }
                        reports::Block::Gap => {
                            ui.separator();
                        }
                    }
                }
            });
        });
}

/// Room for a dialog's text at the current window size.
pub fn dialog_size(ctx: &egui::Context, wanted: Vec2) -> Vec2 {
    let screen = ctx.content_rect().size();
    vec2(wanted.x.min(screen.x - 40.0), wanted.y.min(screen.y - 80.0))
}
