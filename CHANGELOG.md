# [0.4.0]
- New macOS-style dark UI with a sidebar, details panel, toolbar and status bar
- Search and sort projects, with keyboard shortcuts
- Remove projects from the list, and drop a folder on the window to add its projects
- Import the project list from the Unity Hub
- Scanning runs in the background, so the window stays responsive
- A failed launch or a config that can't be read now shows a message instead of crashing or being overwritten
- Show platform modules correctly on macOS
- Fix a startup crash on Linux and a crash on Windows without the Unity Hub
- New app icon, generated from an SVG source
- Removed the light theme
- Project structure: the app is now at the repository root, and the library is in `crates/unity-hub`

# [0.3.0]
- Update egui to 0.37 version
- Update confy to 2, rfd to 0.17, and replace winres with winresource
- Theme switch can now also follow the system theme
- Fix build artifact paths in CI

# [0.2.0]
- Update egui to 0.21 version
- Initial macOS support
- Light theme

# [0.1.0]

- Initial version