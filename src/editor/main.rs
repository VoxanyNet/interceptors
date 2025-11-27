
use std::{fs, path::{Path, PathBuf}};

use interceptors_lib::background::BackgroundSave;
use ldtk2::serde_json;
use macroquad::{miniquad::conf::Platform, window::Conf};

use crate::editor::AreaEditor;

pub mod editor;
pub mod editor_input_context;
pub mod spawner;
pub mod spawner_menu;
pub mod spawner_category;
pub mod selectable_object_id;
pub mod editor_mode_select_ui;
pub mod editor_ui_tick_context;
pub mod layer_toggle_ui;

pub fn list_dir_entries<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
    let path = path.as_ref(); // keep the original path reference
    let entries = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path()) // convert to full PathBuf
        .filter_map(|p: PathBuf| p.to_str().map(|s| s.to_string())) // PathBuf -> String
        .collect();

    Ok(entries)
}

fn round_to_nearest_50(n: f32) -> f32 {
    (n / 50.0).round() * 50.0
}

fn window_conf() -> Conf {
    let conf = Conf {
        window_title: "Interceptors".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        fullscreen: false, 
        platform: Platform::default(),
        ..Default::default()
    };
    //conf.platform.swap_interval = Some(0); // disable vsync
    conf
}

#[macroquad::main(window_conf)]
async fn main() {   

    
    let x = BackgroundSave::default();

    let x = serde_json::to_string_pretty(&x).unwrap();

    let mut area_editor = AreaEditor::new().await;

    area_editor.run().await;
}
