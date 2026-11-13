//! Small building blocks the windows share.

use super::*;
use crate::app::Action;
use crate::data::*;
use crate::app::colors;
use eframe::egui::{self, vec2};
use std::collections::BTreeMap;

pub(crate) fn field(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    ui.end_row();
}

pub(crate) fn citations_editor(
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

pub(crate) fn events_editor(
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
                    .fill(colors::pal().paper)
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
