//! Keyboard shortcuts.

use eframe::egui::{self, Key, Modifiers, Popup};

use super::HubApp;
use crate::{
    commands::{ProjectCmd, ViewCmd},
    state::Page,
    ui::toolbar,
};

impl HubApp {
    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
        if self.state.sheet_open() || Popup::is_any_open(ctx) {
            return;
        }
        let pressed = |modifiers, key| ctx.input_mut(|i| i.consume_key(modifiers, key));

        if pressed(Modifiers::COMMAND, Key::F) {
            self.state.page = Page::Projects;
            self.state.focus_search = true;
        }
        if pressed(Modifiers::COMMAND, Key::N) {
            ProjectCmd::RequestNew(None).schedule();
        }
        if pressed(Modifiers::COMMAND, Key::R) {
            ViewCmd::Refresh.schedule();
        }
        if self.state.page != Page::Projects {
            return;
        }

        // egui has already dropped keyboard focus for Escape by now, so this
        // can't depend on the search box having it.
        if pressed(Modifiers::NONE, Key::Escape) && !self.state.search.is_empty() {
            self.state.search.clear();
            self.state.refresh_visible();
        }
        let search = toolbar::search_id();
        let focused = ctx.memory(egui::Memory::focused);
        // Keys typed into another focused widget belong to it.
        if focused.is_some() && focused != Some(search) {
            return;
        }
        if pressed(Modifiers::NONE, Key::ArrowDown) {
            self.state.select_step(1);
        }
        if pressed(Modifiers::NONE, Key::ArrowUp) {
            self.state.select_step(-1);
        }
        if pressed(Modifiers::NONE, Key::Enter)
            && let Some(path) = self.state.selected.clone()
        {
            ProjectCmd::Open(path).schedule();
        }
    }
}
