//! The eframe application: applies commands in `logic`, lays out panels in
//! `ui`. Each command group is handled in its own module.

mod editors;
mod keys;
mod projects;

use std::path::PathBuf;

use eframe::egui::{self, Frame, Margin, Stroke};
use unity_hub::hub::Hub;

use crate::{
    commands::{Command, ViewCmd},
    jobs::{JobKind, JobResult, Jobs},
    settings::{self, Settings},
    state::{Page, UiState},
    ui::{
        self, confirm_sheet, details_panel, editors_page, projects_page, sidebar, status_bar,
        theme, toasts, toolbar,
    },
};

/// Top-level app.
pub struct HubApp {
    state: UiState,
    jobs: Jobs,
    /// Where the library is saved; tests have nowhere.
    config_path: Option<PathBuf>,
    previous_width: f32,
}

impl HubApp {
    /// Build the app: show the saved library right away, and read the disk
    /// again in the background.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::platform::dialogs::attach(cc);
        theme::configure_theme(&cc.egui_ctx);
        theme::fonts::register_app_fonts(&cc.egui_ctx);

        let settings: Settings = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, settings::KEY))
            .unwrap_or_default();
        let config_path = settings::config_path();
        let (hub, error) = config_path
            .as_deref()
            .map_or_else(|| (Hub::default(), None), settings::load_hub);

        let mut app = Self::build(hub, &settings, config_path);
        app.state.notices.errors.extend(error);
        app.start_refresh(&cc.egui_ctx, JobKind::Startup);
        app
    }

    /// An app showing `hub` that doesn't touch the config, with jobs run
    /// inline and fonts that are the same on every machine.
    #[cfg(test)]
    pub fn with_hub(ctx: &egui::Context, hub: Hub) -> Self {
        theme::configure_theme(ctx);
        theme::fonts::register_test_fonts(ctx);
        let mut app = Self::build(hub, &Settings::default(), None);
        app.jobs.inline = true;
        app
    }

    fn build(hub: Hub, settings: &Settings, config_path: Option<PathBuf>) -> Self {
        Self {
            state: UiState::new(hub, settings),
            jobs: Jobs::default(),
            config_path,
            previous_width: 0.0,
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> &UiState {
        &self.state
    }

    #[cfg(test)]
    pub fn state_mut(&mut self) -> &mut UiState {
        &mut self.state
    }

    fn start_refresh(&mut self, ctx: &egui::Context, kind: JobKind) {
        let mut hub = self.state.hub.clone();
        self.jobs.start(kind, ctx, move || {
            hub.update_data();
            JobResult::Refreshed(Box::new(hub))
        });
    }

    fn save_config(&mut self) {
        let Some(path) = &self.config_path else {
            return;
        };
        if let Err(error) = settings::save_hub(path, &self.state.hub) {
            self.state.notices.errors.push(error);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::Project(command) => self.apply_project(ctx, command),
            Command::Editor(command) => self.apply_editor(ctx, command),
            Command::View(command) => self.apply_view(ctx, command),
        }
    }

    fn apply_view(&mut self, ctx: &egui::Context, command: ViewCmd) {
        let state = &mut self.state;
        match command {
            ViewCmd::ShowPage(page) => state.page = page,
            ViewCmd::Select(path) => state.selected = Some(path),
            ViewCmd::SetSort(sort) => {
                state.sort = sort;
                state.refresh_visible();
            }
            ViewCmd::ToggleSidebar => state.sidebar_visible = !state.sidebar_visible,
            ViewCmd::ToggleDetails => {
                state.details_visible = !state.details_visible;
                state.details_auto_hidden = false;
            }
            ViewCmd::Refresh => self.start_refresh(ctx, JobKind::Refresh),
        }
    }

    fn finish_job(&mut self, kind: JobKind, result: JobResult) {
        match result {
            JobResult::Refreshed(hub) => {
                self.state.hub = *hub;
                self.state.refresh_visible();
                // The launch refresh isn't something the user asked to save.
                if kind == JobKind::Refresh {
                    self.save_config();
                }
            }
            JobResult::Found(projects) => self.add_found_projects(kind, projects),
            JobResult::Failed => self
                .state
                .notices
                .errors
                .push(format!("{} failed", kind.label().trim_end_matches('…'))),
        }
    }

    /// Hide the details panel when the window gets narrow, and bring it back
    /// when it widens, unless the user changed it in between.
    fn auto_collapse_details(&mut self, width: f32) {
        let threshold = ui::DETAILS_AUTO_HIDE_WIDTH;
        let was_wide = self.previous_width <= 0.0 || self.previous_width >= threshold;
        let is_wide = width >= threshold;
        let state = &mut self.state;
        if was_wide && !is_wide && state.details_visible {
            state.details_visible = false;
            state.details_auto_hidden = true;
        } else if !was_wide && is_wide && state.details_auto_hidden {
            state.details_visible = true;
            state.details_auto_hidden = false;
        }
        self.previous_width = width;
    }
}

impl eframe::App for HubApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((kind, result)) = self.jobs.poll() {
            self.finish_job(kind, result);
        }
        for command in Command::drain() {
            self.apply(ctx, command);
        }
        self.state.busy = self.jobs.busy();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, settings::KEY, &self.state.settings());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.auto_collapse_details(ctx.content_rect().width());
        self.handle_keys(&ctx);
        self.handle_dropped_folders(&ctx);

        let bar = |margin: Margin| {
            Frame::new()
                .fill(theme::colors::TOOLBAR_BG)
                .inner_margin(margin)
        };

        egui::Panel::top("toolbar")
            .frame(
                bar(Margin::symmetric(0, 4)).stroke(Stroke::new(1.0, theme::colors::BORDER_SUBTLE)),
            )
            .exact_size(toolbar::TOOLBAR_HEIGHT)
            .show(ui, |ui| toolbar::render(ui, &mut self.state));

        egui::Panel::bottom("status_bar")
            .frame(bar(Margin::symmetric(10, 0)))
            .exact_size(ui::STATUS_BAR_HEIGHT)
            .show(ui, |ui| status_bar::render(ui, &self.state));

        let sidebar_frame = Frame::new()
            .fill(theme::colors::SIDEBAR_BG)
            .inner_margin(Margin::symmetric(10, 10));
        let mut sidebar_visible = self.state.sidebar_visible;
        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(ui::SIDEBAR_WIDTH)
            .frame(sidebar_frame)
            .show_collapsible(ui, &mut sidebar_visible, |ui| {
                sidebar::render(ui, &self.state);
            });
        self.state.sidebar_visible = sidebar_visible;

        // The details belong to the projects page.
        let mut details_visible = self.state.details_visible && self.state.page == Page::Projects;
        egui::Panel::right("details")
            .resizable(true)
            .default_size(280.0)
            .size_range(260.0..=380.0)
            .frame(
                Frame::new()
                    .fill(theme::colors::SIDEBAR_BG)
                    .inner_margin(Margin::symmetric(12, 10)),
            )
            .show_collapsible(ui, &mut details_visible, |ui| {
                details_panel::render(ui, &self.state);
            });

        let page_rect = egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(theme::colors::PANEL_BG)
                    .inner_margin(Margin::same(12)),
            )
            .show(ui, |ui| match self.state.page {
                Page::Projects => projects_page::render(ui, &mut self.state),
                Page::Editors => editors_page::render(ui, &self.state),
            })
            .response
            .rect;

        toasts::render(
            &ctx,
            page_rect,
            std::mem::take(&mut self.state.notices.toasts),
            std::mem::take(&mut self.state.notices.errors),
        );
        confirm_sheet::render(&ctx, &mut self.state);
        Self::show_drop_overlay(&ctx, page_rect);

        // Widgets scheduled commands for `logic`: ask for the frame that
        // applies them, since no input may be coming.
        if Command::pending() {
            ctx.request_repaint();
        }
    }
}
