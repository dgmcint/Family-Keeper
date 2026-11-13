//! The main window's screens and what they share: menu bar, tabs, people list and status bar.

mod duplicates;
mod family;
mod lists;
mod pedigree;
mod report_screen;

use duplicates::*;
use family::*;
use lists::*;
use pedigree::*;
use report_screen::*;

use crate::app::{Action, FamilyApp, Pending, Relation, Tab};
use crate::data::*;
use crate::app::colors;
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke, Vec2, vec2};
use std::collections::HashMap;

pub fn show(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let busy = app.busy();
    egui::Panel::top("menu")
        .frame(egui::Frame::new().fill(colors::pal().card).inner_margin(Margin::symmetric(8, 3)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| menu_bar(app, ui));
        });
    egui::Panel::top("tabs")
        .frame(egui::Frame::new().fill(colors::pal().paper).inner_margin(Margin::symmetric(12, 8)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| tabs_row(app, ui));
        });
    egui::Panel::bottom("status")
        .frame(egui::Frame::new().fill(colors::pal().card).inner_margin(Margin::symmetric(12, 4)))
        .show(ui, |ui| status_bar(app, ui));
    egui::Panel::left("people")
        .resizable(true)
        .default_size(270.0)
        .size_range(200.0..=460.0)
        .frame(egui::Frame::new().fill(colors::pal().card).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| people_list(app, ui));
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors::pal().paper).inner_margin(Margin::same(14)))
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
            ui.label(RichText::new("Text size").color(colors::pal().muted));
            for (zoom, label) in [(1.0, "Normal"), (1.25, "Large"), (1.5, "Extra large")] {
                if ui.selectable_label(app.zoom == zoom, label).clicked() {
                    app.set_zoom(&ctx, zoom);
                    ui.close();
                }
            }
            ui.separator();
            ui.menu_button("Colors", |ui| {
                for (i, (name, group, _)) in colors::SCHEMES.iter().enumerate() {
                    if i == 0 || colors::SCHEMES[i - 1].1 != *group {
                        if i > 0 {
                            ui.separator();
                        }
                        ui.label(RichText::new(*group).color(colors::pal().muted));
                    }
                    if ui.selectable_label(colors::current() == i, *name).clicked() {
                        colors::choose(&ctx, i);
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
        ui.label(RichText::new(&app.status).color(colors::pal().muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let saved = if app.dirty { "Unsaved changes" } else { "All changes saved" };
            let saved = if app.is_sample && !app.dirty { "Sample, not saved" } else { saved };
            ui.label(RichText::new(saved).color(if app.dirty { colors::pal().gold } else { colors::pal().muted }));
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}  ·  {}  ·",
                    count(app.db.people.len(), "person", "people"),
                    count(app.db.families.len(), "marriage", "marriages"),
                    count(app.db.sources.len(), "source", "sources"),
                ))
                .color(colors::pal().muted),
            );
        });
    });
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn sample_banner(app: &mut FamilyApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(colors::pal().gold_soft)
        .stroke(Stroke::new(1.0, colors::pal().border))
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
    ui.label(RichText::new(label).small().color(colors::pal().muted));
    ui.separator();
    if app.rows.is_empty() {
        let text = if total == 0 { "Nobody here yet." } else { "No one matches that search." };
        ui.label(RichText::new(text).color(colors::pal().muted));
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

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().strong().color(colors::pal().muted));
}

fn card_frame(highlight: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(if highlight { colors::pal().accent_soft } else { colors::pal().card })
        .stroke(Stroke::new(if highlight { 1.5 } else { 1.0 }, if highlight { colors::pal().accent } else { colors::pal().border }))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
}

fn fact_line(ui: &mut egui::Ui, label: &str, event: Option<&Event>) {
    let Some(e) = event.filter(|e| !e.summary().is_empty()) else { return };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().interact_size.y = 16.0;
        ui.label(RichText::new(label).color(colors::pal().muted));
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
                if ui.link(RichText::new(p.name()).size(17.0).color(colors::pal().accent)).clicked() {
                    actions.push(Action::Select(p.id));
                }
                person_facts(ui, p);
            }
            None => {
                ui.label(RichText::new("Not recorded").color(colors::pal().muted));
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

fn shorten(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut s: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    s.push('…');
    s
}

fn table_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).strong());
}

/// Room for a dialog's text at the current window size.
pub fn dialog_size(ctx: &egui::Context, wanted: Vec2) -> Vec2 {
    let screen = ctx.content_rect().size();
    vec2(wanted.x.min(screen.x - 40.0), wanted.y.min(screen.y - 80.0))
}
