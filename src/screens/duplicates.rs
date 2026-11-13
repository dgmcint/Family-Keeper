//! The Duplicates screen.

use crate::i18n::tr;
use super::*;
use crate::app::{Action, FamilyApp};
use crate::app::colors;
use eframe::egui::{self, RichText};

pub(crate) fn duplicates_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let (db, actions) = (&app.db, &mut app.actions);
    ui.label(RichText::new(tr("Possible duplicates")).size(18.0).strong());
    ui.label(RichText::new(tr("Finds people with the same name whose birth years don't rule out a match. You choose who to merge.")).color(colors::pal().muted));
    ui.add_space(4.0);
    if ui.button(tr("Check for duplicates")).clicked() {
        actions.push(Action::FindDuplicates);
    }
    ui.add_space(6.0);
    let Some(pairs) = &app.duplicates else { return };
    let ignored = &app.ignored_duplicates;
    let pairs: Vec<&(u32, u32)> = pairs.iter().filter(|p| !ignored.contains(p)).collect();
    if pairs.is_empty() {
        ui.label(tr("No possible duplicates found."));
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
                            ui.label(RichText::new(format!("ID {}", p.id)).small().color(colors::pal().muted));
                            person_facts(ui, p);
                            if ui.button(tr("Keep this one and merge")).clicked() {
                                actions.push(Action::AskMerge { keep: p.id, drop: other.id });
                            }
                        }
                    });
                    if ui.small_button(tr("These are different people")).clicked() {
                        actions.push(Action::IgnoreDuplicate(a, b));
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}
