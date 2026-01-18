use std::path::PathBuf;

use fxhash::FxHashMap;
use macroquad::texture::{self, load_texture, Texture2D};
use web_sys::console;

use crate::normalize_path;

#[derive(Clone)]
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

    pub fn load(&mut self, texture_path: PathBuf, bytes: &[u8]) {

        if !self.cache.contains_key(&texture_path) {

            let texture = Texture2D::from_file_with_format(bytes, None);
            texture.set_filter(texture::FilterMode::Nearest);

            self.cache.insert(texture_path.clone(), texture);

        }
    }
    pub fn get(&self, texture_path: &PathBuf) -> &Texture2D {   

        let normalized_path = normalize_path(texture_path);

        log::debug!("Loading texture: {:?}", normalized_path);

        self.cache.get(&normalized_path).unwrap()

    }

    
}