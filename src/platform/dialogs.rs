//! Native folder dialogs, parented to the app's window.

use std::{cell::RefCell, path::PathBuf};

use rfd::FileDialog;

thread_local! {
    // RFD copies the root window's handles. The root outlives all dialogs,
    // which only open on its UI thread.
    static FILE: RefCell<FileDialog> = RefCell::new(FileDialog::new());
}

/// Parent later dialogs to the app's window.
pub fn attach(cc: &eframe::CreationContext<'_>) {
    FILE.set(FileDialog::new().set_parent(cc));
}

/// Ask for a folder. Blocks until the dialog closes.
#[must_use]
pub fn pick_folder() -> Option<PathBuf> {
    FILE.with_borrow(Clone::clone).pick_folder()
}
