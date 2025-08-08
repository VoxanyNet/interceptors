use macroquad::{color::WHITE, math::Vec2, texture::{draw_texture_ex, DrawTextureParams}};
use nalgebra::vector;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle};
use serde::{Deserialize, Serialize};

use crate::{space::Space, texture_loader::TextureLoader};

// literally just a sprite with position and size
pub struct Decoration {
    pub pos: Vec2, // rapier pos
    pub sprite_path: String,
    pub size: Vec2
}

impl Decoration {
    pub fn from_save(&mut self, save: &mut DecorationSave) -> Self {
        
        Self {
            pos: save.pos,
            sprite_path: save.sprite_path.clone(),
            size: save.size
        }
    }

    pub fn save(&self) -> DecorationSave {
        DecorationSave {
            pos: self.pos,
            size: self.size,
            sprite_path: self.sprite_path.clone(),
        }
    }
    
    pub async fn draw(&self, textures: &mut TextureLoader) {
        let texture = textures.get(&self.sprite_path).await;

        draw_texture_ex(
            texture, 
            self.pos.x, 
            self.pos.y, 
            WHITE, 
            DrawTextureParams {
                dest_size: Some(self.size),
                source: None,
                rotation: 0.,
                flip_x: false,
                flip_y: false,
                pivot: None,
            }
        );
    }
}

#[derive(Serialize, Deserialize)]
pub struct DecorationSave {
    pub pos: Vec2,
    pub size: Vec2,
    pub sprite_path: String
}