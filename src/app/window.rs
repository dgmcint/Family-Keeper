//! Keyboard shortcuts, the window title, zoom and closing the window.

use super::*;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, ViewportCommand};

impl FamilyApp {
    pub(crate) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let cmd = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let shift = |key| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key);
        let pressed = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));
        if pressed(shift(Key::S)) {
            self.save_as();
        } else if pressed(cmd(Key::S)) {
            self.save();
        } else if pressed(cmd(Key::O)) {
            self.request(Pending::Open, ctx);
        } else if pressed(cmd(Key::N)) {
            self.request(Pending::NewFile, ctx);
        } else if !ctx.egui_wants_keyboard_input() {
            // While typing in the search box these keys belong to the typing.
            if pressed(shift(Key::Z)) {
                self.redo();
            } else if pressed(cmd(Key::Z)) {
                self.undo();
            }
        }
    }

    pub(crate) fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.person_editor.is_some() || self.family_editor.is_some() || self.source_editor.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.status = "Finish or cancel the open window first.".into();
            } else if self.dirty {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                if self.confirm.is_none() {
                    self.confirm = Some(Confirm::Unsaved(Pending::Quit));
                }
            }
        }
    }

    pub(crate) fn update_title(&mut self, ctx: &egui::Context) {
        let title = format!("{}{} – Family Keeper", if self.dirty { "• " } else { "" }, self.file_label());
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        self.zoom = zoom;
        ctx.set_zoom_factor(zoom);
    }
}
