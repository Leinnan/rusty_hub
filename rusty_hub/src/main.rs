use crate::config::Configuration;
#[macro_use]
extern crate serde_derive;
extern crate confy;

mod config;
mod unity_editor;
mod unity_project;

fn main() {
    let config = Configuration::default();
    println!("{:#?}", config.editors_configurations);
    let projects = unity_project::UnityProject::get_projects_from_registry();
    println!("{:#?}", projects);
}
