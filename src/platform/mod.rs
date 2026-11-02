//! What differs between operating systems.

pub mod dialogs;

use std::{io, process::Command};

#[cfg(windows)]
const FILE_MANAGER: &str = "explorer";
#[cfg(target_os = "macos")]
const FILE_MANAGER: &str = "open";
#[cfg(target_os = "linux")]
const FILE_MANAGER: &str = "xdg-open";

/// What menus call showing a folder in the file manager.
#[cfg(windows)]
pub const REVEAL_LABEL: &str = "Show in Explorer";
/// What menus call showing a folder in the file manager.
#[cfg(target_os = "macos")]
pub const REVEAL_LABEL: &str = "Show in Finder";
/// What menus call showing a folder in the file manager.
#[cfg(target_os = "linux")]
pub const REVEAL_LABEL: &str = "Show in Files";

/// Show the folder at `path` in the file manager.
pub fn reveal(path: &str) -> io::Result<()> {
    let mut child = Command::new(FILE_MANAGER).arg(path).spawn()?;
    // Wait from a thread of its own, so it doesn't stay around as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
