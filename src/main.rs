//! Rusty Hub: a small alternative to the Unity Hub.

// Release builds on Windows are GUI apps: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod commands;
mod jobs;
mod platform;
mod settings;
mod state;
mod ui;

#[cfg(test)]
mod tests;

use eframe::egui::ViewportBuilder;

/// Matches the `.desktop` file on Linux, which pairs windows with their launcher.
const APP_ID: &str = "io.github.Leinnan.RustyHub";

/// The window icon, rendered from `assets/icon/icon.svg` by `build.rs`.
const WINDOW_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let icon =
        eframe::icon_data::from_png_bytes(WINDOW_ICON).expect("the bundled icon is a valid PNG");
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(ui::APP_NAME)
            .with_app_id(APP_ID)
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([760.0, 480.0])
            .with_icon(icon)
            // macOS: extend content under a transparent title bar so the app
            // toolbar and the traffic-light buttons share one unified bar.
            .with_fullsize_content_view(cfg!(target_os = "macos"))
            .with_titlebar_shown(!cfg!(target_os = "macos"))
            .with_title_shown(!cfg!(target_os = "macos")),
        // The app id would otherwise also choose where eframe keeps its
        // settings, and everything saved so far would be left behind.
        persistence_path: eframe::storage_dir(ui::APP_NAME).map(|dir| dir.join("app.ron")),
        ..Default::default()
    };
    eframe::run_native(
        ui::APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(app::HubApp::new(cc)))),
    )
}
