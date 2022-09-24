#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use rusty_hub::hub::Hub;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rusty Hub",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../static/FiraCode-VF.ttf")),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

struct MyApp {
    text: String,
    hub: Hub,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        Self {
            text: "Edit this text field if you want".to_owned(),
            hub: Hub::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("some_unique_id")
                .striped(true)
                .min_row_height(30.0)
                .max_col_width(500.0)
                .show(ui, |ui| {
                    let mut index: usize = 0;
                    for project in &self.hub.projects {
                        if self.hub.editor_for_project(project).is_some() {
                            if ui.button("run").clicked() {
                                self.hub.run_project_nr(index);
                            }
                        }
                        ui.label(&project.title);
                        let version_response =
                            ui.add(egui::Label::new(&project.version).sense(egui::Sense::click()));
                        version_response.context_menu(|ui| {
                            for editor in &self.hub.config.editors_configurations {
                                if ui.button(format!("Open in {}", editor.version)).clicked() {
                                    Hub::run_project(&editor, &project);
                                    ui.close_menu();
                                }
                            }
                        });
                        let path_response =
                            ui.add(egui::Label::new(&project.path).sense(egui::Sense::click()));
                        path_response.context_menu(|ui| {
                            if ui.button("Open directory").clicked() {
                                use std::process::Command;
                                Command::new("explorer").arg(&project.path).spawn().unwrap();
                                ui.close_menu();
                            }
                        });
                        ui.end_row();
                        index = index + 1;
                    }
                });
        });
    }
}
