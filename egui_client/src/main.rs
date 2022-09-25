#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
extern crate confy;
use eframe::{egui, IconData, NativeOptions};
use std::io::Cursor;

mod consts;
mod hub_client;
mod window_tab;

fn main() {
    let img = image::io::Reader::new(Cursor::new(include_bytes!("../static/hub.png")))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    let icon = IconData {
        width: 32,
        height: 32,
        rgba: img.into_rgba8().into_raw(),
    };
    let options = NativeOptions {
        always_on_top: false,
        maximized: false,
        decorated: true,
        fullscreen: false,
        drag_and_drop_support: false,
        initial_window_size: Some(egui::vec2(820.0, 400.0)),
        min_window_size: Some(egui::vec2(420.0, 400.0)),
        icon_data: Some(icon),
        ..NativeOptions::default()
    };
    eframe::run_native(
        "Rusty Hub",
        options,
        Box::new(|cc| Box::new(crate::hub_client::HubClient::new(cc))),
    );
}
