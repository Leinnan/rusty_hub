//! The editors page.

use eframe::egui::Role;
use egui_kittest::kittest::Queryable;
use unity_hub::{config::Configuration, hub::Hub};

use super::{editor, harness, harness_with, settle};
use crate::state::Page;

fn open_editors(harness: &mut egui_kittest::Harness<'static, crate::app::HubApp>) {
    settle(harness);
    harness
        .get_by_role_and_label(Role::Button, "Editors")
        .click();
    settle(harness);
    assert_eq!(harness.state().state().page, Page::Editors);
}

#[test]
fn editors_page_snapshot() {
    let mut harness = harness();
    open_editors(&mut harness);
    harness.snapshot("editors");
}

#[test]
fn lists_search_paths_and_editors_with_platforms() {
    let mut harness = harness();
    open_editors(&mut harness);
    assert!(
        harness
            .query_by_label("/Applications/Unity/Hub/Editor")
            .is_some()
    );
    assert!(harness.query_by_label("Android").is_some());
    assert!(harness.query_by_label("iOS").is_some());
    assert!(
        harness.query_by_role(Role::TextInput).is_none(),
        "search belongs to the projects page"
    );
}

#[test]
fn removing_a_search_path_refreshes_the_editors() {
    let mut harness = harness();
    open_editors(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Remove /Applications/Unity/Hub/Editor")
        .click();
    settle(&mut harness);
    let state = harness.state().state();
    assert!(state.hub.config.unity_search_paths.is_empty());
    assert!(state.hub.config.editors_configurations.is_empty());
    assert!(
        harness
            .query_by_label("No editors found in the search paths.")
            .is_some()
    );
}

#[test]
fn an_empty_page_says_what_to_do() {
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: Vec::new(),
        },
        Vec::new(),
    );
    let mut harness = harness_with(hub, 1100.0, 700.0);
    open_editors(&mut harness);
    assert!(harness.query_by_label_contains("No search paths").is_some());
}

#[test]
fn platforms_that_do_not_fit_collapse_into_a_count() {
    let many = [
        "Android", "iOS", "macOS", "Linux", "Windows", "WebGL", "UWP", "tvOS", "visionOS",
        "Switch", "PS5", "Xbox",
    ];
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![editor("6000.2.8f1", &many)],
        },
        Vec::new(),
    );
    let mut harness = harness_with(hub, 760.0, 480.0);
    open_editors(&mut harness);
    assert!(harness.query_by_label_contains("more platforms").is_some());
}
