//! The Reports screen. (Named so it is not confused with the reports themselves, in src/reports.rs.)

use crate::app::{Action, FamilyApp};
use crate::reports;
use crate::app::colors;
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke};

pub(crate) fn reports_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
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
        ui.label(RichText::new("Choose a person first.").color(colors::pal().muted));
        return;
    };
    egui::Frame::new()
        .fill(colors::pal().card)
        .stroke(Stroke::new(1.0, colors::pal().border))
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
