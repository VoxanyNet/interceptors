use std::path::PathBuf;

use fxhash::FxHashMap;
use image::{DynamicImage, ImageReader, load_from_memory};
use macroquad::texture::{self, load_texture, Texture2D};
use web_sys::console;

use crate::normalize_path;


#[derive(Clone)]
pub struct ServerTextureLoader {
    pub cache: FxHashMap<PathBuf, DynamicImage>
}

impl Default for ServerTextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerTextureLoader {

    pub fn new() -> Self {
        ServerTextureLoader { cache: FxHashMap::default() }
    }

    pub fn load(&mut self, texture_path: PathBuf, bytes: &[u8]) {

        if !self.cache.contains_key(&texture_path) {

            let texture = load_from_memory(bytes).unwrap();

            self.cache.insert(texture_path.clone(), texture);

        }
    }
    pub fn get(&self, texture_path: &PathBuf) -> &DynamicImage  {   

        let normalized_path = normalize_path(texture_path);

        //log::debug!("Loading texture: {:?}", normalized_path);

        self.cache.get(&normalized_path).unwrap()

    }

    
}