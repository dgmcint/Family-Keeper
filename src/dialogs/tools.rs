//! Renumbering records.

use super::*;
use crate::app::{Confirm, FamilyApp};
use crate::app::colors;
use eframe::egui::{self, RichText};

pub(crate) fn renumber_window(app: &mut FamilyApp, ctx: &egui::Context) -> Option<Answer> {
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
        ui.label(RichText::new("Each person keeps their old ID, shown on their card and written to exported GEDCOM files. Undo puts everything back.").color(colors::pal().muted));
        ui.add_space(6.0);
        let order = db.renumber_order(crate::app::renumber_how(*how));
        ui.label(RichText::new("First people after renumbering:").strong());
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Grid::new("renum").striped(true).num_columns(3).show(ui, |ui| {
                ui.label(RichText::new("New ID").color(colors::pal().muted));
                ui.label(RichText::new("Now").color(colors::pal().muted));
                ui.label(RichText::new("Name").color(colors::pal().muted));
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
