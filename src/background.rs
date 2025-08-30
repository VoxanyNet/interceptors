use std::path::PathBuf;

use macroquad::{camera, color::WHITE, math::{Rect, Vec2}, texture::{draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::texture_loader::TextureLoader;

#[derive(Clone)]
pub struct Background {
    pub repeat: bool,
    pub pos: Vec2,
    pub sprite_path: PathBuf,
    pub size: Vec2,
    pub parallax: f32
}

impl Background {

    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect) {
        let texture = textures.get(&self.sprite_path);

        //set_default_camera();

        draw_texture_ex(
            texture, 
            self.pos.x + camera_rect.x * self.parallax, 
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

        if self.repeat {
            for x in -20..20 {
                draw_texture_ex(
                    texture, 
                    (self.pos.x + (x as f32 * self.size.x)) + camera_rect.x * self.parallax, 
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

        // let mut camera = Camera2D::from_display_rect(*camera_rect);
        // camera.zoom.y = -camera.zoom.y;

        // set_camera(&camera);

    }

    pub fn save(&self) -> BackgroundSave {

        BackgroundSave {
            repeat: self.repeat,
            pos: self.pos,
            sprite_path: self.sprite_path.clone(),
            size: self.size,
            parallax: self.parallax
        }
    }

    pub fn from_save(save: BackgroundSave) -> Self {
        Self {
            repeat: save.repeat,
            pos: save.pos,
            sprite_path: save.sprite_path,
            size: save.size,
            parallax: save.parallax
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BackgroundSave {
    repeat: bool,
    pos: Vec2,
    sprite_path: PathBuf,
    size: Vec2,
    parallax: f32
}