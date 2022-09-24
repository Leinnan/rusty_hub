use unity_editor::UnityEditor;

use crate::config::Configuration;
#[macro_use]
extern crate serde_derive;
extern crate confy;

mod config;
mod unity_editor;

fn main() {
    let config = Configuration::default();
    let paths = config.get_unity_paths();
    for path in &paths {
        let editor = UnityEditor::new(&path);
        if editor.is_some() {
            println!("{:#?}", editor.unwrap());
        }
    }
}
