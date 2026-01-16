use std::path::PathBuf;

use fxhash::FxHashMap;
use macroquad::audio::{Sound, load_sound, load_sound_from_bytes, stop_sound};

use crate::normalize_path;

#[derive(Clone)]
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

    pub async fn load(&mut self, sound_path: PathBuf, bytes: &[u8]) {

        if !self.cache.contains_key(&sound_path) {

            let sound = load_sound_from_bytes(bytes).await.unwrap();

            self.cache.insert(sound_path, sound);
        }
    }
    pub fn get(&self, sound_path: PathBuf) -> &Sound {

        let normalized_path = normalize_path(&sound_path);

        self.cache.get(&normalized_path).unwrap()


    }
}
