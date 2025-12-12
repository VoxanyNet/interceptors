use std::path::PathBuf;

use macroquad::{color::WHITE, math::{Rect, Vec2}, texture::{draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::{drawable::{DrawContext, Drawable}, texture_loader::TextureLoader};

#[derive(Clone)]
pub struct Background {
    pub repeat: bool,
    pub pos: Vec2,
    pub sprite_path: PathBuf,
    pub size: Vec2,
    pub parallax: f32
}

impl Background {


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

#[async_trait::async_trait]
impl Drawable for Background {
    async fn draw(&mut self, draw_context: &DrawContext) {
        let texture = draw_context.textures.get(&self.sprite_path);

        //set_default_camera();

        //draw_context.camera_rect.y - texture.height(), 
        draw_texture_ex(
            texture, 
            self.pos.x + draw_context.camera_rect.x * self.parallax, 
            self.pos.y * self.parallax, 
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

                for y in -5..5 {
                    draw_texture_ex(
                    texture, 
                    (self.pos.x + (x as f32 * self.size.x)) + draw_context.camera_rect.x * self.parallax, 
                    (self.pos.y + (y as f32 * self.size.y)) + draw_context.camera_rect.y * self.parallax, 
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
        }

        // let mut camera = Camera2D::from_display_rect(*camera_rect);
        // camera.zoom.y = -camera.zoom.y;

        // set_camera(&camera);
    }

    fn draw_layer(&self) -> u32 {
        0
    }
}
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct BackgroundSave {
    repeat: bool,
    pos: Vec2,
    sprite_path: PathBuf,
    size: Vec2,
    parallax: f32
}