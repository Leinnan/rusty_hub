//! egui front end: theme, widgets and the window's panels.

pub mod confirm_sheet;
pub mod details_panel;
pub mod editors_page;
pub mod format;
pub mod projects_page;
pub mod sidebar;
pub mod start_screen;
pub mod status_bar;
pub mod theme;
pub mod toasts;
pub mod toolbar;
pub mod widgets;

/// The app's name.
pub const APP_NAME: &str = "Rusty Unity Hub";

/// The app's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the project lives.
pub const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

/// Height of the status bar (px).
pub const STATUS_BAR_HEIGHT: f32 = 24.0;

/// Width of the left sidebar (px).
pub const SIDEBAR_WIDTH: f32 = 190.0;

/// Window width below which the details panel auto-hides, like a macOS split
/// view collapsing its sidebar.
pub const DETAILS_AUTO_HIDE_WIDTH: f32 = 900.0;
