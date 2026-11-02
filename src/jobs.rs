//! One background job at a time, for work that walks the disk.
//!
//! The app disables anything that would change the library while a job runs,
//! so a job's result can't overwrite a newer change.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;
use unity_hub::{hub::Hub, unity_project::UnityProject};

/// What a job is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Refreshing the library at launch.
    Startup,
    /// Refreshing the library on request.
    Refresh,
    /// Looking for projects in a folder.
    Scan,
    /// Reading the Unity Hub's project list.
    Import,
}

impl JobKind {
    /// What the status bar says while it runs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Startup | Self::Refresh => "Refreshing…",
            Self::Scan => "Scanning…",
            Self::Import => "Importing…",
        }
    }
}

/// What a job produced.
#[derive(Debug)]
pub enum JobResult {
    /// The library, read from disk again.
    Refreshed(Box<Hub>),
    /// Projects found.
    Found(Vec<UnityProject>),
    /// The job died without a result.
    Failed,
}

enum Running {
    Thread(JobKind, Receiver<JobResult>),
    /// Run on the calling thread, for tests.
    #[cfg_attr(not(test), allow(dead_code))]
    Done(JobKind, JobResult),
}

/// The job runner.
#[derive(Default)]
pub struct Jobs {
    running: Option<Running>,
    /// Run jobs on the calling thread, so tests see results on the next frame.
    #[cfg(test)]
    pub inline: bool,
}

impl Jobs {
    /// The job running, if any.
    #[must_use]
    pub fn busy(&self) -> Option<JobKind> {
        match self.running.as_ref()? {
            Running::Thread(kind, _) | Running::Done(kind, _) => Some(*kind),
        }
    }

    /// Start `work` unless a job is already running.
    pub fn start(
        &mut self,
        kind: JobKind,
        ctx: &egui::Context,
        work: impl FnOnce() -> JobResult + Send + 'static,
    ) {
        if self.running.is_some() {
            return;
        }
        #[cfg(test)]
        if self.inline {
            self.running = Some(Running::Done(kind, work()));
            ctx.request_repaint();
            return;
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // The receiver only goes away when the app is closing.
            let _ = tx.send(work());
            ctx.request_repaint();
        });
        self.running = Some(Running::Thread(kind, rx));
    }

    /// The finished job's result, if there is one.
    pub fn poll(&mut self) -> Option<(JobKind, JobResult)> {
        match self.running.take()? {
            Running::Done(kind, result) => Some((kind, result)),
            Running::Thread(kind, rx) => match rx.try_recv() {
                Ok(result) => Some((kind, result)),
                Err(TryRecvError::Disconnected) => Some((kind, JobResult::Failed)),
                Err(TryRecvError::Empty) => {
                    self.running = Some(Running::Thread(kind, rx));
                    None
                }
            },
        }
    }
}
