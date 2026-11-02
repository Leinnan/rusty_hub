//! What is remembered between sessions: the library in `config.toml`, and
//! layout preferences in eframe's storage.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use unity_hub::hub::Hub;

use crate::state::SortKey;

/// eframe storage key.
pub const KEY: &str = "rusty_hub.settings";

/// Layout preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Project order.
    pub sort: SortKey,
    /// Left sidebar shown.
    pub sidebar_visible: bool,
    /// Details panel shown.
    pub details_visible: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sort: SortKey::default(),
            sidebar_visible: true,
            details_visible: true,
        }
    }
}

/// Location of the saved library.
///
/// This is the path confy 0.6 used, under the app's original name. Newer
/// confy versions default to a different directory, and the app has since
/// been renamed, so it is set explicitly to keep existing configs loading.
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("rs", "", "rusty_hub_egui")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// The library saved at `path`, creating it if there is none. A file that
/// can't be read is copied aside rather than lost to the next save, and an
/// empty library comes back with a message saying so.
#[must_use]
pub fn load_hub(path: &Path) -> (Hub, Option<String>) {
    match confy::load_path::<Hub>(path) {
        Ok(hub) => (hub, None),
        Err(error) => {
            let backup = path.with_extension("toml.bak");
            let kept = match std::fs::copy(path, &backup) {
                Ok(_) => format!("A copy is at {}.", backup.display()),
                Err(_) => "It couldn't be copied.".to_owned(),
            };
            log::error!("reading {}: {error}", path.display());
            let message = format!("Couldn't read your saved projects. {kept}");
            (Hub::default(), Some(message))
        }
    }
}

/// Save the library to `path`.
pub fn save_hub(path: &Path, hub: &Hub) -> Result<(), String> {
    confy::store_path(path, hub).map_err(|error| format!("Couldn't save your projects: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use unity_hub::hub::Hub;

    use super::{load_hub, save_hub};

    #[test]
    fn keeps_the_path_older_versions_saved_to() {
        let path = super::config_path().expect("a config folder");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("config.toml")
        );
        assert!(path.to_string_lossy().contains("rusty_hub_egui"));
    }

    #[test]
    fn a_saved_library_loads_back() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let path = tmp.path().join("config.toml");
        let mut hub = Hub::default();
        hub.hidden_projects.push("/tmp/Removed".to_owned());
        save_hub(&path, &hub).expect("saved");

        let (loaded, error) = load_hub(&path);
        assert_eq!(error, None);
        assert_eq!(loaded.hidden_projects, ["/tmp/Removed"]);
    }

    #[test]
    fn a_broken_config_is_kept_and_reported() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let path = tmp.path().join("config.toml");
        fs::write(&path, "not [valid toml").expect("written");

        let (hub, error) = load_hub(&path);
        assert!(hub.projects.is_empty());
        assert!(error.expect("a message").contains("config.toml.bak"));
        assert_eq!(
            fs::read_to_string(tmp.path().join("config.toml.bak")).expect("backup"),
            "not [valid toml"
        );
    }
}
