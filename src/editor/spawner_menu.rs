use std::collections::HashMap;

use crate::{list_dir_entries, spawner_category::SpawnerCategory};

/// Contains all the prefab paths for each spawner category
pub struct SpawnerMenu {
    pub prefabs: HashMap<SpawnerCategory, Vec<String>>
}

impl SpawnerMenu {
    pub fn new() -> Self {

        let mut prefabs: HashMap<SpawnerCategory, Vec<String>> = HashMap::new();

        prefabs.insert(SpawnerCategory::Decoration, list_dir_entries("prefabs/decorations/").unwrap());
        prefabs.insert(SpawnerCategory::Background, list_dir_entries("prefabs/backgrounds/").unwrap());        
        prefabs.insert(SpawnerCategory::Prop, list_dir_entries("prefabs/generic_physics_props/").unwrap());  
        prefabs.insert(SpawnerCategory::Tile, list_dir_entries("prefabs/tiles/").unwrap());

        Self {  
            prefabs
        }
    }
}