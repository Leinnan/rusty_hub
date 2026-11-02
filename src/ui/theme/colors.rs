//! App-wide macOS-style dark palette (graphite neutrals + system blue
//! accent). Keep all magic color values here.

use eframe::egui::Color32;

/// Primary label text (macOS `labelColor`).
pub const TEXT: Color32 = Color32::from_rgb(236, 236, 240);
/// Secondary label text (macOS `secondaryLabelColor`).
pub const TEXT_WEAK: Color32 = Color32::from_rgb(168, 168, 174);
/// Placeholder / hint text, visibly dimmer than real input text.
pub const HINT_TEXT: Color32 = Color32::from_rgb(146, 146, 152);
/// Background behind the page content.
pub const PANEL_BG: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x20);
/// Sidebar and details panel background.
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x28);
/// Unified toolbar and status bar background.
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2d);
/// Sheet / popup window background.
pub const WINDOW_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
/// Grouped box and button face fill.
pub const CARD_BG: Color32 = Color32::from_rgb(0x30, 0x30, 0x33);
/// Hovered card / row fill.
pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3d);
/// Pressed button and checkbox well fill.
pub const CONTROL_TRACK: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x4e);
/// Faint striping background.
pub const FAINT_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
/// Text-field fill.
pub const EXTREME_BG: Color32 = Color32::from_rgb(0x12, 0x12, 0x14);
/// Control and window border.
pub const BORDER: Color32 = Color32::from_rgb(0x3d, 0x3d, 0x41);
/// Hairline separators between bars.
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x35, 0x35, 0x38);
/// macOS system blue (dark appearance).
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x0a, 0x84, 0xff);
/// Selected row / segment fill.
pub const SELECTION_BG: Color32 = Color32::from_rgb(0x1d, 0x4e, 0x89);
/// Uppercase section heading color.
pub const SECTION_LABEL: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x93);
/// Soft destructive/error text and borders.
pub const DELETE_RED_SOFT: Color32 = Color32::from_rgb(0xff, 0x6b, 0x61);
/// Fill of buttons that destroy something.
pub const DESTRUCTIVE_BG: Color32 = Color32::from_rgb(0xd0, 0x3b, 0x32);
/// macOS system orange: something needs attention (a missing editor).
pub const WARNING: Color32 = Color32::from_rgb(0xff, 0x9f, 0x0a);
/// macOS system green: ready to go (an installed editor).
pub const SUCCESS: Color32 = Color32::from_rgb(0x32, 0xd7, 0x4b);
/// Dims the page under a folder being dragged over it.
pub const DROP_OVERLAY: Color32 = Color32::from_rgba_premultiplied(5, 25, 45, 200);
