//! What differs between operating systems.

pub mod dialogs;

use std::{io, path::PathBuf, process::Command};

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

/// Unity's own folders on this machine, as `(what to call it, path)`. Only the
/// ones that exist are listed, since not every install has every one.
#[must_use]
pub fn unity_folders() -> Vec<(String, String)> {
    let Some(dirs) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    candidate_folders(&dirs)
        .into_iter()
        .filter(|(_, path)| path.is_dir())
        .map(|(name, path)| (name.to_owned(), path.to_string_lossy().into_owned()))
        .collect()
}

#[cfg(target_os = "macos")]
fn candidate_folders(dirs: &directories::BaseDirs) -> Vec<(&'static str, PathBuf)> {
    let library = dirs.home_dir().join("Library");
    vec![
        ("Editor Logs", library.join("Logs").join("Unity")),
        ("Package Cache", library.join("Unity").join("cache")),
        (
            "Asset Store Packages",
            library.join("Unity").join("Asset Store-5.x"),
        ),
    ]
}

#[cfg(windows)]
fn candidate_folders(dirs: &directories::BaseDirs) -> Vec<(&'static str, PathBuf)> {
    let local = dirs.data_local_dir().join("Unity");
    vec![
        ("Editor Logs", local.join("Editor")),
        ("Package Cache", local.join("cache")),
        (
            "Asset Store Packages",
            dirs.data_dir().join("Unity").join("Asset Store-5.x"),
        ),
    ]
}

#[cfg(target_os = "linux")]
fn candidate_folders(dirs: &directories::BaseDirs) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("Editor Logs", dirs.config_dir().join("unity3d")),
        ("Package Cache", dirs.cache_dir().join("unity3d")),
        (
            "Asset Store Packages",
            dirs.data_dir().join("unity3d").join("Asset Store-5.x"),
        ),
    ]
}

/// Show the folder at `path` in the file manager.
pub fn reveal(path: &str) -> io::Result<()> {
    let mut child = Command::new(FILE_MANAGER).arg(path).spawn()?;
    // Wait from a thread of its own, so it doesn't stay around as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
