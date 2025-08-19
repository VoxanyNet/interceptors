use std::path::PathBuf;

use fxhash::FxHashMap;
use macroquad::texture::{self, load_texture, Texture2D};

pub struct TextureLoader {
    pub cache: FxHashMap<PathBuf, Texture2D>
}

impl Default for TextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureLoader {

    pub fn new() -> Self {
        TextureLoader { cache: FxHashMap::default() }
    }

    pub async fn load(&mut self, texture_path: PathBuf) {

        // this can probably be optimized with a match statement but i cant figure it out the borrowing stuff
        if !self.cache.contains_key(&texture_path) {

            let texture = load_texture(&texture_path.to_string_lossy()).await.unwrap();
            
            texture.set_filter(texture::FilterMode::Nearest);

            self.cache.insert(texture_path.clone(), texture);

        }
    }
    pub fn get(&self, texture_path: &PathBuf) -> &Texture2D {

        self.cache.get(texture_path).unwrap()
    }
}