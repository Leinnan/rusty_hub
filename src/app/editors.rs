//! Editor commands.

use eframe::egui;
use unity_hub::hub::Hub;

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
            EditorCmd::Run(exe) => {
                let state = &mut self.state;
                let Some(editor) = state.editor(&exe) else {
                    return;
                };
                let version = editor.version.clone();
                match Hub::run_editor(editor) {
                    Ok(()) => state
                        .notices
                        .toasts
                        .push(format!("Opening Unity {version}")),
                    Err(error) => state
                        .notices
                        .errors
                        .push(format!("Couldn't open Unity {version}: {error}")),
                }
            }
            EditorCmd::SetPreferred(exe) => {
                if self.state.busy.is_none() {
                    self.state.hub.preferred_editor = exe;
                    self.save_config();
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
