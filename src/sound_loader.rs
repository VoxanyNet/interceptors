use fxhash::FxHashMap;
use macroquad::{audio::{load_sound, Sound}, texture::{self, load_texture, Texture2D}};

pub struct SoundLoader {
    pub cache: FxHashMap<String, Sound>
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

    pub async fn load(&mut self, sound_path: impl ToString) {
        let sound_path = sound_path.to_string();

        if !self.cache.contains_key(&sound_path) {
            let sound = load_sound(&sound_path).await.unwrap();

            self.cache.insert(sound_path, sound);
        }
    }
    pub fn get(&self, sound_path: impl ToString) -> &Sound {

        let sound_path = sound_path.to_string();

        dbg!(&sound_path);

        self.cache.get(&sound_path).unwrap()
    }
}