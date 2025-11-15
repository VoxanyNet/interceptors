use std::path::PathBuf;

use macroquad::{color::WHITE, math::Vec2, texture::{draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::{drawable::Drawable, texture_loader::TextureLoader};

// literally just a sprite with position and size
#[derive(Clone, PartialEq)]
pub struct Decoration {
    pub pos: Vec2, // macroquad pos
    pub sprite_path: Option<PathBuf>,
    pub size: Vec2,
    pub frame_duration: Option<web_time::Duration>,
    pub animated_sprite_paths: Option<Vec<PathBuf>>,
    pub layer: u32
}

impl Decoration {
    pub fn from_save(save: DecorationSave) -> Self {

        let frame_duration = match save.frame_duration {
            Some(dur) => Some(web_time::Duration::from_secs_f32(dur)),
            None => None,
        };
            
        Self {
            pos: save.pos,
            sprite_path: save.sprite_path,
            animated_sprite_paths: save.animated_sprite_paths,
            size: save.size,
            frame_duration,
            layer: save.layer
        }
    }

    pub fn save(&self) -> DecorationSave {

        let frame_duration = match self.frame_duration {
            Some(duration) => Some(duration.as_secs_f32()),
            None => None,
        };
        
        DecorationSave {
            pos: self.pos,
            size: self.size,
            sprite_path: self.sprite_path.clone(),
            animated_sprite_paths: self.animated_sprite_paths.clone(),
            frame_duration: frame_duration,
            layer: self.layer
        }
    }
    
}
#[async_trait::async_trait]
impl Drawable for Decoration {
    async fn draw(&mut self, draw_context: &crate::drawable::DrawContext) {
        let sprite_path = match &self.frame_duration {
            Some(frame_duration) => {
                let current_frame = (
                    (
                        draw_context.elapsed_time.as_secs_f32() % (frame_duration.as_secs_f32() * self.animated_sprite_paths.as_ref().unwrap().len() as f32)
                    ) / frame_duration.as_secs_f32()
                ) as usize;

                &self.animated_sprite_paths.as_ref().unwrap()[current_frame]
            },
            None => {
                self.sprite_path.as_ref().unwrap()
            },
        };

        let texture = draw_context.textures.get(sprite_path);

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

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DecorationSave {
    pub pos: Vec2,
    pub size: Vec2,
    #[serde(default)]
    pub sprite_path: Option<PathBuf>,
    #[serde(default)]
    pub animated_sprite_paths: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub frame_duration: Option<f32>,
    #[serde(default)]
    pub layer: u32
}