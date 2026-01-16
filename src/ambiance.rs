use std::path::PathBuf;

use macroquad::{audio::{PlaySoundParams, Sound, play_sound}, math::Vec2};
use serde::{Deserialize, Serialize};

use crate::sound_loader::SoundLoader;

pub struct Ambiance {
    pub path: PathBuf,
    pub pos: Vec2,
    pub volume: f32,
    pub sound: Option<Sound>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AmbianceSave {
    path: PathBuf,
    pos: Vec2,
    volume: f32
}
impl Ambiance {

    pub fn new(path: PathBuf, pos: Vec2, volume: f32) -> Self {
        Self {
            path: path,
            pos: pos,
            volume: volume,
            sound: None,
        }
    }
    pub fn start_if_stopped(&mut self, sounds: &mut SoundLoader) {
        if self.sound.is_none() {
            let sound = sounds.get(self.path.clone());
            play_sound(
                sound, 
                PlaySoundParams {
                    looped: true,
                    volume: self.volume,
                }
            );

            self.sound = Some(sound.clone());
        }
    }
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