use crate::{
    consts::HOMEPAGE,
    consts::{VERSION, VERTICAL_SPACING},
    window_tab::WindowTab,
};
use eframe::{
    egui::{self, Layout, Ui},
    epaint::Color32,
};
use egui_extras::{Size, TableBuilder};
use rfd::FileDialog;
use rusty_hub::hub::Hub;

pub struct HubClient {
    hub: Hub,
    current_tab: WindowTab,
}

impl HubClient {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
                    ui.add_space(VERTICAL_SPACING);
                });
                header.col(|ui| {
                    ui.heading("Version");
                    ui.add_space(VERTICAL_SPACING);
                });
                header.col(|ui| {
                    ui.heading("Directory");
                    ui.add_space(VERTICAL_SPACING);
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
                            ui.vertical_centered_justified(|ui| {
                                ui.add_space(VERTICAL_SPACING - 2.0);
                                if ui
                                    .add_enabled(
                                        editor_for_project_exists,
                                        egui::Button::new(format!("{}", &project.title)),
                                    )
                                    .on_disabled_hover_text(format!(
                                        "Select different Unity version"
                                    ))
                                    .clicked()
                                {
                                    self.hub.run_project_nr(row_index);
                                }
                                ui.add_space(VERTICAL_SPACING);
                            });
                        });
                        row.col(|ui| {
                            ui.with_layout(
                                Layout::top_down_justified(eframe::emath::Align::Center),
                                |ui| {
                                    ui.add_space(VERTICAL_SPACING);
                                    let mut text = egui::RichText::new(&project.version);
                                    if !editor_for_project_exists {
                                        text = text.color(Color32::RED);
                                    }
                                    let version_response =
                                        ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                    version_response.context_menu(|ui| {
                                        for editor in &self.hub.config.editors_configurations {
                                            let mut text = egui::RichText::new(format!(
                                                "Open in {}",
                                                &editor.version
                                            ));
                                            if editor.version.contains(&project.version) {
                                                text = text.strong().color(Color32::GREEN);
                                            }
                                            if ui.button(text).clicked() {
                                                Hub::run_project(&editor, &project);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                },
                            );
                        });
                        row.col(|ui| {
                            ui.with_layout(
                                Layout::top_down_justified(eframe::emath::Align::Max),
                                |ui| {
                                    ui.add_space(VERTICAL_SPACING);
                                    let path_response = ui.add(
                                        egui::Label::new(&project.path).sense(egui::Sense::click()),
                                    );
                                    path_response.context_menu(|ui| {
                                        if ui.button("Open directory").clicked() {
                                            use std::process::Command;
                                            Command::new("explorer")
                                                .arg(&project.path)
                                                .spawn()
                                                .unwrap();
                                            ui.close_menu();
                                        }
                                    });
                                },
                            );
                        });
                    },
                );
            });
    }
}

impl eframe::App for HubClient {
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
                    ui.add_space(VERTICAL_SPACING);

                    let button =
                        egui::Button::new(egui::RichText::new("Projects").heading()).frame(false);
                    if ui
                        .add_enabled(&self.current_tab != &WindowTab::Projects, button)
                        .clicked()
                    {
                        self.current_tab = WindowTab::Projects;
                    }
                    ui.add_space(VERTICAL_SPACING);
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
            ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.hyperlink_to(format!("v {}", VERSION), HOMEPAGE);
            });
        });
    }
}
