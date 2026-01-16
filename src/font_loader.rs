use std::{collections::HashMap, path::PathBuf};

use macroquad::text::{Font, load_ttf_font, load_ttf_font_from_bytes};

#[derive(Clone)]
pub struct FontLoader {
    fonts: HashMap<PathBuf, Font>
}

impl FontLoader {

    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }

    pub fn load(&mut self, path: PathBuf, bytes: &[u8]) {

        let font = load_ttf_font_from_bytes(bytes).unwrap();

        self.fonts.insert(path, font);

    }
    pub fn get(&self, path: PathBuf) -> Font {
        self.fonts.get(&path).unwrap().clone()
    }
}