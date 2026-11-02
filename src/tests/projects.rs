//! The projects page: list, search, sort, details, removing and adding.

use eframe::egui::{Key, Modifiers, Role};
use egui_kittest::kittest::{NodeT, Queryable};
use unity_hub::{config::Configuration, hub::Hub};

use super::{editor, harness, harness_with, project, sample_hub, settle, visible_titles};
use crate::{commands::ProjectCmd, state::SortKey};

#[test]
fn projects_page_snapshot() {
    let mut harness = harness();
    settle(&mut harness);
    harness.snapshot("projects");
}

#[test]
fn newest_project_comes_first_and_sort_changes_the_order() {
    let mut harness = harness();
    settle(&mut harness);
    assert_eq!(
        visible_titles(&harness),
        ["Mushroom Masters", "Asteroid Miner", "Zen Garden"]
    );

    harness.get_by_role_and_label(Role::Button, "Name").click();
    settle(&mut harness);
    assert_eq!(harness.state().state().sort, SortKey::Name);
    assert_eq!(
        visible_titles(&harness),
        ["Asteroid Miner", "Mushroom Masters", "Zen Garden"]
    );

    harness
        .get_by_role_and_label(Role::Button, "Version")
        .click();
    settle(&mut harness);
    assert_eq!(visible_titles(&harness)[2], "Asteroid Miner", "oldest last");
}

#[test]
fn search_filters_rows_and_counts_them() {
    let mut harness = harness();
    settle(&mut harness);
    let search = harness.get_by_role(Role::TextInput);
    search.focus();
    search.type_text("feature/min");
    settle(&mut harness);
    assert_eq!(visible_titles(&harness), ["Asteroid Miner"]);
    assert!(harness.query_by_label("1 of 3 projects").is_some());
}

#[test]
fn a_search_with_no_matches_says_so() {
    let mut harness = harness();
    settle(&mut harness);
    let search = harness.get_by_role(Role::TextInput);
    search.focus();
    search.type_text("zzz");
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("No projects match")
            .is_some()
    );
}

#[test]
fn selecting_a_row_fills_the_details_panel() {
    let mut harness = harness();
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("Select a project")
            .is_some()
    );

    harness
        .get_by_role_and_label(Role::Button, "Asteroid Miner")
        .click();
    settle(&mut harness);
    assert_eq!(
        harness.state().state().selected.as_deref(),
        Some("/work/games/Asteroid Miner")
    );
    assert!(harness.query_by_label("Not installed").is_some());
    assert_eq!(
        harness.query_all_by_label("feature/mining").count(),
        2,
        "the row's chip and the details panel"
    );
    harness.snapshot("project_selected");
}

#[test]
fn a_missing_editor_shows_a_warning_and_disables_open() {
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: Vec::new(),
        },
        vec![project("Lonely", "2022.3.22f1", "", 1)],
    );
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    assert!(
        harness
            .query_by_label("2022.3.22f1, editor missing")
            .is_some()
    );
    let open = harness.get_by_role_and_label(Role::Button, "Open");
    assert!(open.accesskit_node().is_disabled());
}

#[test]
fn a_failed_launch_shows_an_error_and_keeps_running() {
    let mut harness = harness();
    settle(&mut harness);
    // The fixture's editor executable doesn't exist.
    harness
        .get_all_by_role_and_label(Role::Button, "Open")
        .find(|button| !button.accesskit_node().is_disabled())
        .expect("an enabled Open")
        .click();
    settle(&mut harness);
    assert!(harness.query_by_label_contains("Couldn't open").is_some());
}

#[test]
fn remove_asks_first_and_cancel_keeps_the_project() {
    let mut harness = harness();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Zen Garden")
        .click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Remove from List…")
        .click();
    settle(&mut harness);
    harness.snapshot("confirm_remove");
    assert_eq!(visible_titles(&harness).len(), 3, "asked, not removed");

    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    settle(&mut harness);
    assert_eq!(harness.state().state().confirm_remove, None);
    assert_eq!(visible_titles(&harness).len(), 3);
}

#[test]
fn a_removed_project_stays_gone_after_a_refresh() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let settings = tmp.path().join("Game").join("ProjectSettings");
    std::fs::create_dir_all(&settings).expect("create project");
    std::fs::write(
        settings.join("ProjectVersion.txt"),
        "m_EditorVersion: 6000.0.1f1\n",
    )
    .expect("write version");
    let game = tmp.path().join("Game").to_string_lossy().into_owned();
    let found =
        unity_hub::unity_project::UnityProject::try_get_project_at_path(&game).expect("a project");
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: Vec::new(),
        },
        vec![found],
    );
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Game").click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Remove from List…")
        .click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Remove")
        .click();
    settle(&mut harness);
    assert!(visible_titles(&harness).is_empty());

    harness
        .get_by_role_and_label(Role::Button, "Refresh")
        .click();
    settle(&mut harness);
    assert!(visible_titles(&harness).is_empty(), "stays removed");
    assert_eq!(harness.state().state().hub.hidden_projects, [game]);
}

#[test]
fn adding_a_folder_finds_its_projects_and_says_how_many() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let settings = tmp.path().join("Dropped").join("ProjectSettings");
    std::fs::create_dir_all(&settings).expect("create project");
    std::fs::write(
        settings.join("ProjectVersion.txt"),
        "m_EditorVersion: 6000.0.1f1\n",
    )
    .expect("write version");

    let mut harness = harness();
    settle(&mut harness);
    ProjectCmd::AddFolder(tmp.path().to_path_buf()).schedule();
    settle(&mut harness);
    assert!(visible_titles(&harness).contains(&"Dropped".to_owned()));
    assert!(harness.query_by_label("Added 1 project").is_some());

    // Nothing new the second time.
    ProjectCmd::AddFolder(tmp.path().to_path_buf()).schedule();
    settle(&mut harness);
    assert!(harness.query_by_label("No new projects found").is_some());
}

#[test]
fn the_empty_state_offers_scan_and_import() {
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: Vec::new(),
        },
        Vec::new(),
    );
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    assert!(harness.query_by_label("No projects yet").is_some());
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Scan Folder…")
            .is_some()
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Import from Unity Hub")
            .is_some()
    );
    harness.snapshot("empty_state");
}

#[test]
fn the_details_panel_hides_on_a_narrow_window_and_returns() {
    let mut harness = harness_with(sample_hub(), 760.0, 480.0);
    settle(&mut harness);
    assert!(!harness.state().state().details_visible);
    assert!(harness.state().state().details_auto_hidden);
    harness.snapshot("narrow_window");

    harness.set_size(eframe::egui::vec2(1100.0, 700.0));
    settle(&mut harness);
    assert!(harness.state().state().details_visible);
}

#[test]
fn keyboard_moves_the_selection_and_opens() {
    let mut harness = harness();
    settle(&mut harness);
    harness.key_press(Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(
        harness.state().state().selected.as_deref(),
        Some("/work/games/Mushroom Masters")
    );
    harness.key_press(Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(
        harness.state().state().selected.as_deref(),
        Some("/work/games/Asteroid Miner")
    );
    harness.key_press(Key::ArrowUp);
    settle(&mut harness);
    assert_eq!(
        harness.state().state().selected.as_deref(),
        Some("/work/games/Mushroom Masters")
    );
    harness.key_press(Key::Enter);
    settle(&mut harness);
    assert!(harness.query_by_label_contains("Couldn't open").is_some());
}

#[test]
fn keyboard_focuses_and_clears_search() {
    let mut harness = harness();
    settle(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    settle(&mut harness);
    let search = harness.get_by_role(Role::TextInput);
    assert!(search.is_focused());
    search.type_text("zen");
    settle(&mut harness);
    assert_eq!(visible_titles(&harness), ["Zen Garden"]);

    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert_eq!(harness.state().state().search, "");
    assert_eq!(visible_titles(&harness).len(), 3);
}

#[test]
fn selecting_with_the_keyboard_scrolls_off_screen_rows_into_view() {
    let projects = (0..40)
        .map(|n| project(&format!("Game {n:02}"), "6000.2.8f1", "", n))
        .collect();
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![editor("6000.2.8f1", &[])],
        },
        projects,
    );
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Game 39")
            .is_none(),
        "rows off screen aren't laid out"
    );

    // With nothing selected, going up starts at the bottom.
    harness.key_press(Key::ArrowUp);
    settle(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Game 39")
            .is_some()
    );
}

#[test]
fn long_and_non_ascii_names_do_not_panic() {
    let long = "feature/żółw-".repeat(20);
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: Vec::new(),
        },
        vec![project(&"Długa nazwa ".repeat(30), "6000.2.8f1", &long, 1)],
    );
    let mut harness = harness_with(hub, 760.0, 480.0);
    settle(&mut harness);
    harness.key_press(Key::ArrowDown);
    settle(&mut harness);
    assert!(harness.state().state().selected.is_some());
}

#[test]
fn a_toast_appears_for_a_confirmation() {
    let mut harness = harness();
    settle(&mut harness);
    harness
        .state_mut()
        .state_mut()
        .notices
        .toasts
        .push("Added 2 projects".to_owned());
    settle(&mut harness);
    assert!(harness.query_by_label("Added 2 projects").is_some());
    harness.snapshot("toast");
}
