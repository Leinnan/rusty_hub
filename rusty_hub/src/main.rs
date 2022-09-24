#[macro_use]
extern crate serde_derive;
extern crate confy;

mod config;
mod hub;
mod unity_editor;
mod unity_project;

fn main() {
    let hub = self::hub::Hub::default();
    println!("{:#?}", hub);
    // hub.run_project_nr(0);
}
