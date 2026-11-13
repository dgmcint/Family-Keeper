//! The People, Marriages and Sources lists.

use crate::i18n::{tr, tr_n};
use super::*;
use crate::app::{Action, FamilyApp, SortBy, Tab};
use crate::data::*;
use crate::app::colors;
use eframe::egui::{self, RichText, Sense};
use egui_extras::{Column, TableBuilder};

pub(crate) fn people_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, rows, actions, selected, sort) = (&app.db, &app.rows, &mut app.actions, app.selected, app.sort);
    if rows.is_empty() {
        ui.label(RichText::new(tr("Nobody to show.")).color(colors::pal().muted));
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
                        let text = if sort == by { format!("{} ▾", tr(label)) } else { tr(label).to_string() };
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
                    ui.label(tr(p.sex.label()));
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

pub(crate) fn families_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, actions) = (&app.db, &mut app.actions);
    if db.families.is_empty() {
        ui.label(RichText::new(tr("No marriages or families recorded yet.")).color(colors::pal().muted));
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

pub(crate) fn sources_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    app.refresh_source_use();
    let (db, actions, uses) = (&app.db, &mut app.actions, &app.source_use);
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Sources")).size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr("Add a source")).clicked() {
                actions.push(Action::NewSource);
            }
        });
    });
    ui.add_space(4.0);
    if db.sources.is_empty() {
        ui.label(RichText::new(tr("No sources yet. Sources record where your information came from.")).color(colors::pal().muted));
        return;
    }
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for s in db.sources.values() {
            card_frame(false).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let title = if s.title.trim().is_empty() { tr("(no title)") } else { s.title.as_str() };
                        ui.label(RichText::new(title).strong());
                        if !s.author.is_empty() {
                            ui.label(RichText::new(&s.author).color(colors::pal().muted));
                        }
                        let n = uses.get(&s.id).copied().unwrap_or(0);
                        ui.label(RichText::new(tr_n("Used {n} time", "Used {n} times", n, &[])).small().color(colors::pal().muted));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let used = uses.get(&s.id).copied().unwrap_or(0) > 0;
                        let delete = ui.add_enabled(!used, egui::Button::new(tr("Delete…")));
                        if delete.on_disabled_hover_text(tr("People or events still cite this source.")).clicked() {
                            actions.push(Action::AskDeleteSource(s.id));
                        }
                        if ui.button(tr("Edit")).clicked() {
                            actions.push(Action::EditSource(s.id));
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    });
}
