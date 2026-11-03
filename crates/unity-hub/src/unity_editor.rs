//! An installed Unity editor.

use crate::consts;
use crate::project_template::ProjectTemplate;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct UnityEditor {
    pub version: String,
    pub exe_path: String,
    pub base_path: String,
    pub platforms: Vec<String>,
    pub templates: Vec<ProjectTemplate>,
}

impl std::hash::Hash for UnityEditor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.exe_path.hash(state);
    }
}

impl PartialEq for UnityEditor {
    fn eq(&self, other: &Self) -> bool {
        self.exe_path == other.exe_path
    }
}

impl UnityEditor {
    /// The editor installed at `path`, if there is one.
    #[must_use]
    pub fn new(path: &str) -> Option<Self> {
        Self::with_known_templates(path, &[])
    }

    /// The editor installed at `path`, reusing what is already known about
    /// its templates rather than reading their archives again.
    #[must_use]
    pub fn with_known_templates(path: &str, known: &[ProjectTemplate]) -> Option<Self> {
        let base_path = Path::new(path);
        let exe_path = base_path.join(consts::UNITY_EXE_NAME);
        let meta = std::fs::metadata(&exe_path);
        if !meta.is_ok_and(|meta| meta.is_file()) {
            return None;
        }

        let version = version_from_exe(&exe_path).or_else(|| version_from_folder(base_path))?;

        Some(Self {
            version,
            exe_path: exe_path.to_string_lossy().into_owned(),
            base_path: String::from(path),
            platforms: UnityEditor::get_platforms(path),
            templates: ProjectTemplate::find_templates(path, known),
        })
    }

    /// Display names of the platform modules installed with an editor.
    fn get_platforms(unity_folder: &str) -> Vec<String> {
        let mut platforms: Vec<String> = consts::PLAYBACK_ENGINE_DIRS
            .iter()
            .filter_map(|dir| std::fs::read_dir(Path::new(unity_folder).join(dir)).ok())
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .map(|name| platform_label(&name).map_or(name, str::to_owned))
            .collect();
        platforms.sort();
        platforms.dedup();
        platforms
    }
}

/// The numbers in a Unity version, so 2022.3.9f1 sorts before 2022.3.22f1.
#[must_use]
pub fn version_key(version: &str) -> Vec<u32> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Editors live in a folder named after their version: directly on macOS, and
/// with an `Editor` folder in between on Windows and Linux.
fn version_from_folder(base_path: &Path) -> Option<String> {
    let name = base_path.file_name()?;
    let folder = if name == "Editor" {
        base_path.parent()?.file_name()?
    } else {
        name
    };
    Some(folder.to_string_lossy().into_owned())
}

/// The display name of a `PlaybackEngines` folder.
fn platform_label(folder: &str) -> Option<&'static str> {
    Some(match folder.to_lowercase().as_str() {
        "androidplayer" => "Android",
        "windowsstandalonesupport" => "Windows",
        "linuxstandalone" | "linuxstandalonesupport" => "Linux",
        "osxstandalone" | "macstandalonesupport" => "macOS",
        "webglsupport" => "WebGL",
        "metrosupport" => "UWP",
        "iossupport" => "iOS",
        _ => return None,
    })
}

/// The version stamped into a Windows editor's executable.
#[cfg(windows)]
fn version_from_exe(exe_path: &Path) -> Option<String> {
    use exe::VSVersionInfo;
    use exe::pe::VecPE;

    let image = VecPE::from_disk_file(exe_path).ok()?;
    let info = VSVersionInfo::parse(&image).ok()?;
    let strings = info.string_file_info?;
    let map = strings.children.first()?.string_map().ok()?;
    let version = map.get("ProductVersion")?;
    version.split('_').next().map(str::to_owned)
}

#[cfg(not(windows))]
fn version_from_exe(_exe_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{platform_label, version_from_folder, version_key};

    #[test]
    fn versions_compare_by_number() {
        assert!(version_key("2022.3.22f1") > version_key("2022.3.9f1"));
        assert!(version_key("6000.0.1f1") > version_key("2022.3.22f1"));
        assert!(version_key("").is_empty());
    }

    #[test]
    fn the_version_comes_from_the_right_folder() {
        for path in ["/hub/6000.2.8f1", "/hub/6000.2.8f1/Editor"] {
            assert_eq!(
                version_from_folder(Path::new(path)).as_deref(),
                Some("6000.2.8f1")
            );
        }
    }

    #[test]
    fn platform_names_match_any_case() {
        assert_eq!(platform_label("iOSSupport"), Some("iOS"));
        assert_eq!(platform_label("LinuxStandalone"), Some("Linux"));
        assert_eq!(platform_label("MacStandaloneSupport"), Some("macOS"));
        assert_eq!(platform_label("Something"), None);
    }
}
