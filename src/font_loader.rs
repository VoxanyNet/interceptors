use std::{collections::HashMap, path::PathBuf};

use macroquad::text::{load_ttf_font, Font};

pub struct FontLoader {
    fonts: HashMap<PathBuf, Font>
}

impl FontLoader {

    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }

    pub async fn load(&mut self, path: PathBuf) {
        let font = load_ttf_font(path.to_str().unwrap()).await.unwrap();

        self.fonts.insert(path, font);

    }
    pub fn get(&self, path: PathBuf) -> Font {
        self.fonts.get(&path).unwrap().clone()
    }
}