//! Editor commands.

use eframe::egui;

use super::HubApp;
use crate::{commands::EditorCmd, jobs::JobKind, platform};

impl HubApp {
    pub(super) fn apply_editor(&mut self, ctx: &egui::Context, command: EditorCmd) {
        match command {
            EditorCmd::AddSearchPath => {
                if self.state.busy.is_none()
                    && let Some(folder) = platform::dialogs::pick_folder()
                {
                    let folder = folder.to_string_lossy().into_owned();
                    let paths = &mut self.state.hub.config.unity_search_paths;
                    if !paths.contains(&folder) {
                        paths.push(folder);
                    }
                    self.start_refresh(ctx, JobKind::Refresh);
                }
            }
            EditorCmd::RemoveSearchPath(path) => {
                if self.state.busy.is_none() {
                    self.state
                        .hub
                        .config
                        .unity_search_paths
                        .retain(|known| *known != path);
                    self.start_refresh(ctx, JobKind::Refresh);
                }
            }
            EditorCmd::Reveal(path) => {
                if let Err(error) = platform::reveal(&path) {
                    self.state
                        .notices
                        .errors
                        .push(format!("Couldn't show {path}: {error}"));
                }
            }
        }
    }
}
