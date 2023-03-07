use crate::{
    consts::HOMEPAGE,
    consts::{APP_NAME, TOP_BUTTON_WIDTH, VERSION, VERTICAL_SPACING, TOP_SIDE_MARGIN},
    window_tab::WindowTab,
};
use eframe::{
    egui::{self, Layout, Ui},
    epaint::{Color32, FontId, FontFamily},
};
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;
use unity_hub_lib::{consts::FILE_MANAGER, hub::Hub};
use inline_tweak::*;

pub struct HubClient {
    hub: Hub,
    current_tab: WindowTab,
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "regular".to_owned(),
        egui::FontData::from_static(include_bytes!("..\\static\\Inter-Regular.ttf")),
    );
    fonts.font_data.insert(
        "semibold".to_owned(),
        egui::FontData::from_static(include_bytes!("..\\static\\Inter-SemiBold.ttf")),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "regular".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Name("semibold".into()))
        .or_default()
        .insert(0, "semibold".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("regular".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

impl HubClient {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        let hub_option = confy::load("rusty_hub_egui", "config");

        let hub = if hub_option.is_ok() {
            let mut h: Hub = hub_option.unwrap();
            h.update_data();
            h
        } else {
            Hub::default()
        };

        let client = Self {
            hub,
            current_tab: WindowTab::Projects,
        };

        client
    }

    fn save_config(&mut self, rebuild: bool) {
        if rebuild {
            self.hub.update_data();
        }
        let _ = confy::store("rusty_hub_egui", "config", &self.hub);
    }

    pub fn draw_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_tab {
                    WindowTab::Projects => self.draw_project(&ctx, ui),
                    WindowTab::Editors => self.draw_editors(&ctx, ui),
                };
            });
        });
    }

    fn draw_editors(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        ui.label(egui::RichText::new("Editor search paths").heading());
        ui.add_space(VERTICAL_SPACING);
        let text_height = egui::TextStyle::Body.resolve(&ui.style()).size * 2.0;

        let paths = self.hub.config.unity_search_paths.clone();
        for (i, path) in paths.iter().enumerate() {
            ui.horizontal(
                |ui| {
                    ui.label(path);
                    let height = tweak!(30.0);
                    let button_width = tweak!(100.0);
                    ui.allocate_space(egui::vec2(ui.available_width()-button_width-TOP_SIDE_MARGIN,height));
                    if ui.add_sized([button_width, height], egui::Button::new("🚮 Remove")).clicked() {
                        self.hub.config.unity_search_paths.remove(i);
                        self.save_config(true);
                        return;
                    }
                });
        }
        ui.add_space(VERTICAL_SPACING * 2.0);

        ui.label(egui::RichText::new("Installed editor versions").heading());
        ui.add_space(VERTICAL_SPACING);

        let table2 = TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(100.0).at_least(100.0).at_most(120.0))
            .column(Column::initial(150.0).at_least(150.0).at_most(400.0))
            .column(Column::remainder().at_least(260.0))
            .resizable(false);

        table2.body(|body| {
            body.rows(
                text_height,
                self.hub.config.editors_configurations.len(),
                |row_index, mut row| {
                    let editor = &self.hub.config.editors_configurations[row_index];
                    row.col(|ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(VERTICAL_SPACING);
                            ui.label(&editor.version);
                        });
                    });
                    row.col(|ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(VERTICAL_SPACING);
                            ui.label(egui::RichText::new(editor.platforms.join(",")).small());
                        });
                    });
                    row.col(|ui| {
                        ui.with_layout(
                            Layout::top_down_justified(eframe::emath::Align::Max),
                            |ui| {
                                ui.add_space(VERTICAL_SPACING);
                                let version_response = ui.add(
                                    egui::Label::new(&editor.base_path).sense(egui::Sense::click()),
                                );
                                version_response.context_menu(|ui| {
                                    let text = egui::RichText::new("🗁 Open directory");
                                    if ui.button(text).clicked() {
                                        use std::process::Command;
                                        Command::new(FILE_MANAGER)
                                            .arg(&editor.base_path)
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
    fn draw_project(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 2.0;

        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(150.0).at_least(150.0))
            .column(Column::initial(90.0).at_least(40.0))
            .column(Column::initial(90.0).at_least(90.0))
            .column(Column::remainder().at_least(260.0))
            .resizable(false);

        table
            .header(25.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Name");
                    ui.add_space(VERTICAL_SPACING);
                });
                header.col(|ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Version");
                        ui.add_space(VERTICAL_SPACING);
                    });
                });
                header.col(|ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Branch");
                        ui.add_space(VERTICAL_SPACING);
                    });
                });
                header.col(|ui| {
                    ui.with_layout(
                        Layout::top_down_justified(eframe::emath::Align::Max),
                        |ui| {
                            ui.heading("Directory");
                            ui.add_space(VERTICAL_SPACING);
                        },
                    );
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
                                    if project.branch.len() < 1 {
                                        return;
                                    }
                                    ui.add_space(VERTICAL_SPACING);
                                    const MAX_BRANCH_LEN: usize = 15;
                                    let is_long = project.branch.len() > MAX_BRANCH_LEN;
                                    let short = if !is_long {
                                        project.branch.clone()
                                    } else {
                                        let mut result =
                                            String::from(&project.branch[0..MAX_BRANCH_LEN]);
                                        result.push_str("...");
                                        result
                                    };

                                    let label = ui.label(egui::RichText::new(short).small());
                                    if is_long {
                                        label.on_hover_text(format!(" {}", &project.branch));
                                    }
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
                                        if ui.button("🗁 Open directory").clicked() {
                                            use std::process::Command;
                                            Command::new(FILE_MANAGER)
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

    fn draw_editors_header(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        add_header(ui);
        if ui
            .add_sized(
                [TOP_BUTTON_WIDTH, 30.0],
                egui::Button::new("🖴 Add new path"),
            )
            .on_hover_text("Add new editor search path")
            .clicked()
        {
            let directory = FileDialog::new().pick_folder();
            if let Some(dir) = directory {
                self.hub
                    .config
                    .unity_search_paths
                    .push(dir.into_os_string().into_string().unwrap());
                self.save_config(true);
            }
        }
        ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, 10.0));
    }

    fn draw_project_header(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        add_header(ui);
        if ui
            .add_sized(
                [TOP_BUTTON_WIDTH, 30.0],
                egui::Button::new("🔭 Scan for projects"),
            )
            .on_hover_text("Scan selected folder for projects")
            .clicked()
        {
            let directory = FileDialog::new().pick_folder();

            if let Some(dir) = directory {
                let amount = self.hub.search_for_projects_at_path(&dir);
                let mut message = rfd::MessageDialog::new().set_title("Search ended");

                match amount {
                    0 => {
                        message = message
                            .set_description("No new projects found.")
                            .set_level(rfd::MessageLevel::Warning)
                    }
                    1 => message = message.set_description("Project founded!"),
                    _ => {
                        message = message.set_description(&format!("Founded {} projects.", amount))
                    }
                }
                message.show();
                self.save_config(true);
            }
        }
        ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, 10.0));
    }

    fn tab_button(&self, ui: &mut Ui, tab: &WindowTab, text: &str) -> bool {
        let button_size = tweak!(36.0);
        let font_size = tweak!(16.0);
        let button_size = egui::vec2(ui.available_width(), button_size);

        let rich_text = if &self.current_tab == tab { egui::RichText::new(text).strong() } else { egui::RichText::new(text).weak() }.font(FontId::new(font_size, FontFamily::Proportional));

        ui.add_sized(button_size, egui::Label::new(rich_text).wrap(false).sense(egui::Sense::click()))
                    .clicked()
    }

    fn draw_side_panel(&mut self, ui: &mut Ui) {
        ui.with_layout(
            Layout::top_down(eframe::emath::Align::Min),
            |ui| {
                if self.tab_button(ui, &WindowTab::Projects, tweak!("📦 Projects"))
                {
                    self.current_tab = WindowTab::Projects;
                }
                if self.tab_button(ui, &WindowTab::Editors, tweak!("🛠 Editors"))
                {
                    self.current_tab = WindowTab::Editors;
                }
            },
        );
    }
}

fn add_header(ui: &mut Ui) {
    let header_height = tweak!(45.0);
    let text =
        egui::RichText::new(APP_NAME).font(FontId::new(26.0, FontFamily::Name("semibold".into()))).strong();
    ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, header_height));
    ui.add(egui::Label::new(text));
    let available_width = ui.available_width() - TOP_BUTTON_WIDTH - TOP_SIDE_MARGIN - TOP_SIDE_MARGIN;
    ui.allocate_space(egui::vec2(available_width, header_height));
}

impl eframe::App for HubClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topPanel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center).with_cross_align(eframe::emath::Align::Center),
                    |ui| {
                        match self.current_tab {
                            WindowTab::Projects => self.draw_project_header(&ctx, ui),
                            WindowTab::Editors => self.draw_editors_header(&ctx, ui),
                        };
                    },
                );
            });

        egui::SidePanel::left("dsadsa")
            .resizable(false)
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                self.draw_side_panel(ui);
            });
        egui::TopBottomPanel::bottom("bottomPanel").show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.hyperlink_to(
                    format!("{} v {}", egui::special_emojis::GITHUB, VERSION),
                    HOMEPAGE,
                );
            });
        });
        self.draw_central_panel(&ctx);
    }
}
