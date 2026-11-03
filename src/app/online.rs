//! What the New Project dialog asks for in the background: Unity's catalog,
//! template downloads, and whether git works. These stay apart from
//! [`crate::jobs`], whose single slot locks the library: a long download
//! shouldn't stop anything else.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::time::Duration;

use eframe::egui;
use unity_hub::{
    new_project::git_available,
    online_templates::{self, OnlineTemplate, base_version},
};

use super::HubApp;
use crate::state::{CatalogStatus, TemplateDownload, TemplateSource};

/// How often to redraw while a download's size changes.
const PROGRESS_REPAINT: Duration = Duration::from_millis(200);

/// What a background request produced.
#[derive(Debug)]
pub enum OnlineResult {
    /// The catalog for an editor version.
    Catalog {
        /// The base version it was asked for, so a late reply can be ignored.
        base: String,
        /// The templates, or why not.
        result: Result<Vec<OnlineTemplate>, String>,
    },
    /// A download finished.
    Downloaded {
        /// The package it was for.
        package: String,
        /// Where it is, or why not.
        result: Result<std::path::PathBuf, String>,
    },
    /// Whether `git` works.
    GitChecked(bool),
}

/// Delivers results from background requests.
pub struct OnlineRunner {
    sender: Sender<OnlineResult>,
    receiver: Receiver<OnlineResult>,
    /// Tests never reach the network: they deliver results themselves.
    #[cfg(test)]
    pub offline: bool,
}

impl Default for OnlineRunner {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            #[cfg(test)]
            offline: false,
        }
    }
}

impl OnlineRunner {
    fn start(&self, ctx: &egui::Context, work: impl FnOnce() -> OnlineResult + Send + 'static) {
        #[cfg(test)]
        if self.offline {
            return;
        }
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // The receiver only goes away when the app is closing.
            let _ = sender.send(work());
            ctx.request_repaint();
        });
    }
}

impl HubApp {
    /// Apply finished requests, and start what the open dialog now needs.
    pub(super) fn update_online(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.online.receiver.try_recv() {
            self.finish_online(result);
        }
        let Some(form) = &self.state.new_project else {
            // Closing the dialog stops its download.
            if let Some(download) = &self.state.online.download {
                download.cancel.store(true, Ordering::Relaxed);
            }
            return;
        };
        if self.state.online.download.is_some() {
            ctx.request_repaint_after(PROGRESS_REPAINT);
        }
        if !self.git_check_started {
            self.git_check_started = true;
            self.online
                .start(ctx, || OnlineResult::GitChecked(git_available()));
        }
        if form.source != TemplateSource::Online {
            return;
        }
        let Some(version) = self.state.editor(&form.editor).map(|e| e.version.clone()) else {
            return;
        };
        let base = base_version(&version);
        if self.state.online.base.as_deref() != Some(base.as_str()) {
            self.state.online.base = Some(base.clone());
            self.state.online.status = CatalogStatus::Loading;
            self.online.start(ctx, move || OnlineResult::Catalog {
                result: online_templates::fetch(&version),
                base,
            });
        }
    }

    /// Apply a finished request.
    pub(super) fn finish_online(&mut self, result: OnlineResult) {
        let online = &mut self.state.online;
        match result {
            OnlineResult::GitChecked(available) => self.state.git_available = Some(available),
            OnlineResult::Catalog { base, result } => {
                // The editor changed while this was on its way.
                if online.base.as_deref() != Some(base.as_str()) {
                    return;
                }
                online.status = match result {
                    Ok(templates) => CatalogStatus::Loaded(templates),
                    Err(message) => CatalogStatus::Failed(message),
                };
            }
            OnlineResult::Downloaded { package, result } => {
                online.download = None;
                match result {
                    Ok(path) => {
                        // Only if the dialog is still there to want it.
                        if let Some(form) = &mut self.state.new_project {
                            form.template = Some(path.to_string_lossy().into_owned());
                        }
                    }
                    Err(message) if message == "Cancelled" => {}
                    Err(message) => {
                        online.download_error = Some((package, message));
                    }
                }
            }
        }
    }

    /// Start downloading a catalog entry, unless one is already on its way.
    pub(super) fn download_template(&mut self, ctx: &egui::Context, package: &str) {
        let online = &mut self.state.online;
        let CatalogStatus::Loaded(templates) = &online.status else {
            return;
        };
        let (Some(template), Some(cache), None) = (
            templates
                .iter()
                .find(|t| t.package_name == package)
                .cloned(),
            online.cache_dir.clone(),
            &online.download,
        ) else {
            return;
        };
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        online.download_error = None;
        online.download = Some(TemplateDownload {
            package: package.to_owned(),
            progress: Arc::clone(&progress),
            cancel: Arc::clone(&cancel),
        });
        self.online.start(ctx, move || OnlineResult::Downloaded {
            package: template.package_name.clone(),
            result: online_templates::download(&template, &cache, &progress, &cancel),
        });
    }

    /// Delete downloaded templates, forgetting one the dialog had picked.
    pub(super) fn clear_template_downloads(&mut self) {
        let Some(cache) = self.state.online.cache_dir.clone() else {
            return;
        };
        if let Some(form) = &mut self.state.new_project
            && form
                .template
                .as_deref()
                .is_some_and(|path| std::path::Path::new(path).starts_with(&cache))
        {
            form.template = None;
        }
        match online_templates::clear_downloads(&cache) {
            Ok(_) => self
                .state
                .notices
                .toasts
                .push("Downloaded templates cleared".to_owned()),
            Err(error) => self
                .state
                .notices
                .errors
                .push(format!("Couldn't clear downloaded templates: {error}")),
        }
    }
}
