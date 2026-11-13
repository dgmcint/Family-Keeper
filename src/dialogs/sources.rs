//! Editing a source.

use super::*;
use crate::app::FamilyApp;
use eframe::egui::{self, RichText};

pub(crate) fn source_window(app: &mut FamilyApp, ctx: &egui::Context) {
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
