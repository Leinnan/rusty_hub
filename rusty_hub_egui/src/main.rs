#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use consts::{APP_NAME, VERSION};
use eframe::egui;

mod consts;
mod hub_client;
mod window_tab;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 400.0])
            .with_min_inner_size([850.0, 400.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../static/hub.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        &format!("{} v {}", APP_NAME, VERSION),
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(crate::hub_client::HubClient::new(cc)))
        }),
    )
}
