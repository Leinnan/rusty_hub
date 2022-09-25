#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
extern crate confy;
use eframe::{
    egui::{self, Layout, Ui},
    epaint::Color32,
    IconData, NativeOptions,
};
use egui_extras::{Size, TableBuilder};
use rfd::FileDialog;
use rusty_hub::hub::Hub;
use std::io::Cursor;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

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
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

#[derive(PartialEq)]
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
                            WindowTab::Projects => (),
                            WindowTab::Editors => self.draw_editors(&ctx, ui),
                        };
                    });

                match self.current_tab {
                    WindowTab::Projects => self.draw_project(&ctx, ui),
                    WindowTab::Editors => (),
                };
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
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 2.0;

        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Size::initial(150.0).at_least(150.0))
            .column(Size::initial(90.0).at_least(40.0))
            .column(Size::remainder().at_least(260.0))
            .resizable(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Version");
                });
                header.col(|ui| {
                    ui.heading("Directory");
                });
            })
            .body(|body| {
                body.rows(
                    text_height,
                    self.hub.projects.len(),
                    |row_index, mut row| {
                        let project = &self.hub.projects[row_index];
                        let editor_for_project_exists =
                            self.hub.editor_for_project(project).is_some();
                        row.col(|ui| {
                            ui.set_enabled(editor_for_project_exists);
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .button(format!("{}", &project.title))
                                    .on_disabled_hover_text(format!(
                                        "Select different Unity version"
                                    ))
                                    .clicked()
                                {
                                    self.hub.run_project_nr(row_index);
                                }
                            });
                            ui.set_enabled(true);
                        });
                        row.col(|ui| {
                            ui.with_layout(
                                Layout::top_down_justified(eframe::emath::Align::Center),
                                |ui| {
                                    let mut text = egui::RichText::new(&project.version);
                                    if !editor_for_project_exists {
                                        text = text.color(Color32::RED);
                                    }
                                    let version_response =
                                        ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                    version_response.context_menu(|ui| {
                                        for editor in &self.hub.config.editors_configurations {
                                            if ui
                                                .button(format!("Open in {}", &editor.version))
                                                .clicked()
                                            {
                                                Hub::run_project(&editor, &project);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                },
                            );
                        });
                        row.col(|ui| {
                            let path_response =
                                ui.add(egui::Label::new(&project.path).sense(egui::Sense::click()));
                            path_response.context_menu(|ui| {
                                if ui.button("Open directory").clicked() {
                                    use std::process::Command;
                                    Command::new("explorer").arg(&project.path).spawn().unwrap();
                                    ui.close_menu();
                                }
                            });
                        });
                    },
                );
            });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topPanel").show(ctx, |ui| {
            ui.add_space(5.0);
            let text = egui::RichText::new(" Rusty Unity Hub").heading().strong();
            ui.add(egui::Label::new(text));
            ui.add_space(5.0);
        });
        egui::SidePanel::left("dsadsa")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(5.0);

                    let button =
                        egui::Button::new(egui::RichText::new("Projects").heading()).frame(false);
                    if ui
                        .add_enabled(&self.current_tab != &WindowTab::Projects, button)
                        .clicked()
                    {
                        self.current_tab = WindowTab::Projects;
                    }
                    ui.add_space(5.0);
                    if ui
                        .add_enabled(
                            &self.current_tab != &WindowTab::Editors,
                            egui::Button::new(egui::RichText::new("Editors").heading())
                                .frame(false),
                        )
                        .clicked()
                    {
                        self.current_tab = WindowTab::Editors;
                    }
                });
            });
        self.draw_central_panel(&ctx);
        egui::TopBottomPanel::bottom("bottomPanel").show(ctx, |ui| {
            ui.with_layout(Layout::top_down(eframe::emath::Align::Max), |ui| {
                ui.hyperlink_to(format!("v {}", VERSION), HOMEPAGE)
            });
        });
    }
}
