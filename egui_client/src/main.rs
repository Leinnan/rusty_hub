#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
extern crate confy;
use eframe::{
    egui::{self, Ui},
    IconData, NativeOptions,
};
use rfd::FileDialog;
use rusty_hub::hub::Hub;
use std::io::Cursor;

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
        icon_data: Some(icon),
        ..NativeOptions::default()
    };
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

pub enum WindowTab {
    Projects,
    Editors,
}
struct MyApp {
    hub: Hub,
    current_tab: WindowTab,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        //setup_custom_fonts(&cc.egui_ctx);
        Self {
            hub: confy::load("lwa_unity_hub", "config").unwrap(),
            current_tab: WindowTab::Projects,
        }
    }

    fn save_config(&mut self, rebuild: bool) {
        if rebuild {
            self.hub.config.rebuild();
        }
        let _ = confy::store("lwa_unity_hub", "config", &self.hub);
    }

    pub fn draw_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("some_unique_id")
                    .striped(true)
                    .min_row_height(45.0)
                    .min_col_width(150.0)
                    .max_col_width(1000.0)
                    .num_columns(3)
                    .show(ui, |ui| {
                        match self.current_tab {
                            WindowTab::Projects => self.draw_project(&ctx, ui),
                            WindowTab::Editors => self.draw_editors(&ctx, ui),
                        };
                    });
            });
        });
    }

    fn draw_editors(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        ui.label("Editor search paths");
        if ui.button("Add new").clicked() {
            let directory = FileDialog::new().pick_folder();
            if let Some(dir) = directory {
                self.hub
                    .config
                    .unity_search_paths
                    .push(dir.into_os_string().into_string().unwrap());
                self.save_config(true);
            }
        }
        if ui.button("Refresh").clicked() {
            self.hub.config.rebuild();
        }
        ui.end_row();
        let mut i = 0;
        for editor in self.hub.config.unity_search_paths.clone() {
            if ui.button("Remove").clicked() {
                self.hub.config.unity_search_paths.remove(i);
                self.save_config(true);
                return;
            }
            ui.label(editor);
            ui.end_row();
            i = i + 1;
        }
        ui.label("Installed editor versions");
        ui.end_row();
        for editor in &self.hub.config.editors_configurations {
            ui.label(&editor.version);
            ui.label(editor.platforms.join(","));
            ui.label(&editor.base_path);
            ui.end_row();
        }
    }
    fn draw_project(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        let mut index: usize = 0;
        for project in &self.hub.projects {
            ui.scope(|ui| {
                ui.set_enabled(self.hub.editor_for_project(project).is_some());
                if ui
                    .button(format!("Open {}", &project.title))
                    .on_disabled_hover_text(format!("Select different Unity version"))
                    .clicked()
                {
                    self.hub.run_project_nr(index);
                }
                ui.set_enabled(true);
            });

            let version_response =
                ui.add(egui::Label::new(&project.version).sense(egui::Sense::click()));
            version_response.context_menu(|ui| {
                for editor in &self.hub.config.editors_configurations {
                    if ui.button(format!("Open in {}", &editor.version)).clicked() {
                        Hub::run_project(&editor, &project);
                        ui.close_menu();
                    }
                }
            });
            let path_response = ui.add(egui::Label::new(&project.path).sense(egui::Sense::click()));
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
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topPanel").show(ctx, |ui| {
            ui.label("Rusty Unity Hub");
        });
        egui::SidePanel::left("dsadsa").show(ctx, |ui| {
            ui.set_height(25.0);
            ui.add_space(14.0);
            if ui.button("Projects").clicked() {
                self.current_tab = WindowTab::Projects;
            }
            ui.add_space(14.0);
            if ui.button("Editors").clicked() {
                self.current_tab = WindowTab::Editors;
            }
        });
        self.draw_central_panel(&ctx);
    }
}
