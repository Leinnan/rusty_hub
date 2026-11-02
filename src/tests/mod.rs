//! Headless UI tests driving the whole app through `egui_kittest`, grouped by
//! area. Shared harness helpers live here.

mod editors;
mod projects;

use std::time::{Duration, SystemTime};

use egui_kittest::Harness;
use unity_hub::{
    config::Configuration, hub::Hub, unity_editor::UnityEditor, unity_project::UnityProject,
};

use crate::app::HubApp;

const DAY: u64 = 86_400;

pub(super) fn project(title: &str, version: &str, branch: &str, days_ago: u64) -> UnityProject {
    UnityProject {
        path: format!("/work/games/{title}"),
        title: title.to_owned(),
        version: version.to_owned(),
        branch: branch.to_owned(),
        is_valid: true,
        edit_time: SystemTime::now() - Duration::from_secs(days_ago * DAY),
    }
}

pub(super) fn editor(version: &str, platforms: &[&str]) -> UnityEditor {
    UnityEditor {
        version: version.to_owned(),
        // Launching a fixture fails, which is what the tests want.
        exe_path: format!("/nonexistent/{version}/Unity"),
        base_path: format!("/Applications/Unity/Hub/Editor/{version}"),
        platforms: platforms.iter().map(|&p| p.to_owned()).collect(),
        templates: Vec::new(),
    }
}

/// A library with an installed editor, a project for it, and two more.
pub(super) fn sample_hub() -> Hub {
    Hub::new(
        Configuration {
            unity_search_paths: vec!["/Applications/Unity/Hub/Editor".to_owned()],
            editors_configurations: vec![editor("6000.2.8f1", &["Android", "iOS", "macOS"])],
        },
        vec![
            project("Mushroom Masters", "6000.2.8f1", "main", 2),
            project("Asteroid Miner", "2022.3.22f1", "feature/mining", 40),
            project("Zen Garden", "6000.2.8f1", "", 400),
        ],
    )
}

pub(super) fn harness_with(hub: Hub, width: f32, height: f32) -> Harness<'static, HubApp> {
    Harness::builder()
        .with_size([width, height])
        .build_eframe(move |cc| HubApp::with_hub(&cc.egui_ctx, hub))
}

/// The sample library at the default window size.
pub(super) fn harness() -> Harness<'static, HubApp> {
    harness_with(sample_hub(), 1100.0, 700.0)
}

#[test]
fn the_window_icon_decodes() {
    let icon = eframe::icon_data::from_png_bytes(crate::WINDOW_ICON).expect("a valid PNG");
    assert_eq!((icon.width, icon.height), (512, 512));
}

/// Run a few frames. Toasts animate, so the UI may not settle by itself.
pub(super) fn settle(harness: &mut Harness<'static, HubApp>) {
    harness.run_steps(4);
}

/// Titles of the visible rows, in order.
pub(super) fn visible_titles(harness: &Harness<'static, HubApp>) -> Vec<String> {
    let state = harness.state().state();
    state
        .visible
        .iter()
        .map(|&ix| state.hub.projects[ix].title.clone())
        .collect()
}
