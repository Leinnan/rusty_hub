//! Section headers.

use eframe::egui::{RichText, Ui};

use crate::ui::theme::colors::SECTION_LABEL;

/// Small uppercase group title, like Finder's "Favorites".
pub fn section_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .strong()
            .color(SECTION_LABEL),
    );
}
