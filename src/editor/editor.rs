use std::collections::HashMap;

use interceptors_lib::{area::Area, texture_loader::TextureLoader};
use macroquad::{color::WHITE, file::load_string, text::draw_text, window::next_frame};
use serde::{Deserialize, Serialize};
use strum::Display;


pub enum Mode {
    Sprite,

}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Display)]
pub enum SpawnerCategory {
    Decoration
}

// this menu is loaded from disk
// for each category there is a list of paths to serialized prefabs of a selected object type
#[derive(Serialize, Deserialize)]
pub struct SpawnerMenu {
    pub menu: HashMap<SpawnerCategory, Vec<String>>
}

pub struct Spawner {
    category: SpawnerCategory,
    menu: SpawnerMenu,
    selected_option: usize // vector index
}

impl Spawner {
    pub async fn new() -> Self {
        let spawner_menu_json = load_string("spawn_menu.json").await.unwrap();

        let spawner_menu: SpawnerMenu = serde_json::from_str(&spawner_menu_json).unwrap();

        Self {
            category: SpawnerCategory::Decoration,
            menu: spawner_menu,
            selected_option: 0
        }



    }

    pub fn tick(&mut self) {
        self.selected_option += 1;

        if self.selected_option > self.menu.menu.get(&self.category).unwrap().len() - 1 {
            self.selected_option = 0
        }
    }

    pub async fn draw(&self) {

        println!("drawing");
        draw_text(&format!("{}", self.category), 0., 20., 24., WHITE);

        for (index, path) in self.menu.menu.get(&self.category).unwrap().get(self.selected_option).iter().enumerate() {

            draw_text(&path, 0., ((index) * 10) as f32 + 40., 20., WHITE);
        }
    }
}
pub struct LevelEditor {
    area: Area,
    textures: TextureLoader,
    spawner: Spawner
    
}

impl LevelEditor {
    pub async fn new() -> Self {

        let textures = TextureLoader::new();
        let spawner = Spawner::new().await;

        Self {
            area: Area::empty(),
            textures,
            spawner
        }
    }

    pub async fn draw(&mut self) {
        self.area.draw(&mut self.textures).await;

        self.spawner.draw().await;

        next_frame().await
    }

    pub fn tick(&mut self) {
        self.spawner.tick();
    }

    pub async fn run(&mut self) {

        loop {
            self.draw().await
        }
        
    } 
}