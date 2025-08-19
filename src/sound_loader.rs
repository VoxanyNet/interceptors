use std::path::PathBuf;

use fxhash::FxHashMap;
use macroquad::{audio::{load_sound, Sound}, texture::{self, load_texture, Texture2D}};

pub struct SoundLoader {
    pub cache: FxHashMap<PathBuf, Sound>
}

impl Default for SoundLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundLoader {

    pub fn new() -> Self {
        SoundLoader { cache: FxHashMap::default() }
    }

    pub async fn load(&mut self, sound_path: PathBuf) {

        if !self.cache.contains_key(&sound_path) {
            let sound = load_sound(&sound_path.to_string_lossy()).await.unwrap();

            self.cache.insert(sound_path, sound);
        }
    }
    pub fn get(&self, sound_path: PathBuf) -> &Sound {

        self.cache.get(&sound_path).unwrap()
    }
}