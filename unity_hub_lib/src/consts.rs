#[cfg(windows)]
pub const UNITY_EXE_NAME: &str = "Unity.exe";
#[cfg(target_os = "macos")]
pub const UNITY_EXE_NAME: &str = "Unity.app/Contents/MacOS/Unity";
#[cfg(target_os = "linux")]
pub const UNITY_EXE_NAME: &str = "Unity";

#[cfg(windows)]
pub const SLASH: &str = "\\";
#[cfg(unix)]
pub const SLASH: &str = "/";
#[cfg(windows)]
pub const FILE_MANAGER: &str = "explorer";
#[cfg(target_os = "macos")]
pub const FILE_MANAGER: &str = "open";
#[cfg(target_os = "linux")]
pub const FILE_MANAGER: &str = "xdg-open";

#[cfg(windows)]
pub const TEMPLATES_DIR: &str = "Data\\Resources\\PackageManager\\ProjectTemplates";
#[cfg(target_os = "macos")]
pub const TEMPLATES_DIR: &str = "Contents/Resources/PackageManager/ProjectTemplates";
#[cfg(target_os = "linux")]
pub const TEMPLATES_DIR: &str = "Data/Resources/PackageManager/ProjectTemplates";
