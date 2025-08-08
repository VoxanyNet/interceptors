use std::collections::HashMap;

use ldtk2::serde_json;
use macroquad::{miniquad::conf::Platform, window::Conf};

use crate::editor::{LevelEditor, SpawnerCategory, SpawnerMenu};

pub mod editor;

fn window_conf() -> Conf {
    let mut conf = Conf {
        window_title: "Interceptors".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        fullscreen: false, 
        platform: Platform::default(),
        ..Default::default()
    };
    conf.platform.swap_interval = Some(0); // disable vsync
    conf
}

#[macroquad::main(window_conf)]
async fn main() {   

    
    let mut area_editor = LevelEditor::new().await;

    area_editor.run().await;
}
