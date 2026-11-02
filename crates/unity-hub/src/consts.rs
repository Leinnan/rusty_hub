//! Per-platform locations.

#[cfg(windows)]
pub const UNITY_EXE_NAME: &str = "Unity.exe";
#[cfg(target_os = "macos")]
pub const UNITY_EXE_NAME: &str = "Unity.app/Contents/MacOS/Unity";
#[cfg(target_os = "linux")]
pub const UNITY_EXE_NAME: &str = "Unity";

#[cfg(windows)]
pub const TEMPLATES_DIR: &str = "Data\\Resources\\PackageManager\\ProjectTemplates";
#[cfg(target_os = "macos")]
pub const TEMPLATES_DIR: &str = "Unity.app/Contents/Resources/PackageManager/ProjectTemplates";
#[cfg(target_os = "linux")]
pub const TEMPLATES_DIR: &str = "Data/Resources/PackageManager/ProjectTemplates";

/// Folders under an editor's install that hold its platform modules.
#[cfg(not(target_os = "macos"))]
pub const PLAYBACK_ENGINE_DIRS: &[&str] = &["Data/PlaybackEngines"];
/// Folders under an editor's install that hold its platform modules.
#[cfg(target_os = "macos")]
pub const PLAYBACK_ENGINE_DIRS: &[&str] =
    &["PlaybackEngines", "Unity.app/Contents/PlaybackEngines"];
