//! The Pedigree screen.

use super::*;
use crate::app::{Action, FamilyApp, Tab};
use crate::app::colors;
use eframe::egui::{self, RichText, Sense, Stroke, StrokeKind, vec2};

pub(crate) fn pedigree_tab(app: &mut FamilyApp, ui: &mut egui::Ui) {
    let Some(root) = app.selected else {
        ui.label(RichText::new("Choose a person to see their ancestors.").color(colors::pal().muted));
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Ancestors of {}", app.db.name_of(root))).size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for g in [5, 4, 3] {
                ui.selectable_value(&mut app.pedigree_generations, g, format!("{g}"));
            }
            ui.label(RichText::new("Generations").color(colors::pal().muted));
        });
    });
    ui.label(RichText::new("Click someone to move them to the front. Double-click to see their family.").small().color(colors::pal().muted));
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
        let line = Stroke::new(1.5, colors::pal().border);
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
                painter.rect(rect, 6.0, colors::pal().paper, Stroke::new(1.0, colors::pal().border), StrokeKind::Inside);
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Not recorded", egui::FontId::proportional(13.0), colors::pal().muted);
                continue;
            };
            let response = ui.interact(rect, ui.scope_id().with(("pedigree", n)), Sense::click());
            let is_root = n == 1;
            let fill = if is_root {
                colors::pal().accent_soft
            } else if response.hovered() {
                colors::pal().card_hover
            } else {
                colors::pal().card
            };
            let edge = if is_root || response.hovered() { colors::pal().accent } else { colors::pal().border };
            painter.rect(rect, 6.0, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
            let left = rect.left() + 10.0;
            painter.text(
                egui::pos2(left, rect.top() + 16.0),
                egui::Align2::LEFT_CENTER,
                shorten(&p.name(), chars),
                egui::FontId::proportional(14.5),
                colors::pal().ink,
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
                colors::pal().muted,
            );
            if response.double_clicked() {
                actions.push(Action::Open(p.id, Tab::Family));
            } else if response.clicked() {
                actions.push(Action::Select(p.id));
            }
        }
    });
}
