#[cfg(windows)]
pub const UNITY_EXE_NAME: &str = "Unity.exe";
#[cfg(target_os = "macos")]
pub const UNITY_EXE_NAME: &str = "Unity.app/Contents/MacOS/Unity";
#[cfg(target_os = "linux")]
pub const UNITY_EXE_NAME: &str = "Unity";
