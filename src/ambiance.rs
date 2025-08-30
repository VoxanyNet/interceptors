use std::path::PathBuf;

use macroquad::{audio::Sound, math::Vec2};
use serde::{Deserialize, Serialize};

pub struct Ambiance {
    pub path: PathBuf,
    pub pos: Vec2,
    pub volume: f32,
    pub sound: Option<Sound>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AmbianceSave {
    path: PathBuf,
    pos: Vec2,
    volume: f32
}
impl Ambiance {
    pub fn from_save(save: AmbianceSave) -> Self {
        Self {
            path: save.path,
            pos: save.pos,
            volume: save.volume,
            sound: None
            
        }
    }

    pub fn save(&self) -> AmbianceSave {
        AmbianceSave {
            path: self.path.clone(),
            pos: self.pos,
            volume: self.volume,
        }
    }
}