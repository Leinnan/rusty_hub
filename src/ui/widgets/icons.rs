//! Lucide icon glyphs. Lucide is the last fallback of the Proportional font
//! chain, so icon codepoints render without naming the font family.

use eframe::egui::RichText;
pub use lucide_icons::Icon;

/// `RichText` rendering `icon` with the Lucide font.
#[must_use]
pub fn glyph(icon: Icon) -> RichText {
    RichText::new(char::from(icon))
}
