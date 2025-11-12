use std::path::PathBuf;

use macroquad::{color::{Color, WHITE}, input::{is_mouse_button_down, is_mouse_button_released}, math::{Rect, Vec2}, texture::{DrawTextureParams, draw_texture_ex}};

use crate::texture_loader::TextureLoader;


#[derive(Debug, Clone)]
pub struct Button {
    pub hovered: bool,
    pub down: bool,
    pub released: bool,
    pub rect: Rect,
    pub image: Option<PathBuf>,
}

impl Button {

    pub fn new(rect: Rect, image:Option<PathBuf>) -> Self {
        Self {
            hovered: false,
            down: false,
            released: false,
            rect,
            image
        }
    }

    pub fn draw(&self, textures: &TextureLoader) {

        if let Some(image) = &self.image {

            let texture = textures.get(image);
            draw_texture_ex(
                texture, 
                self.rect.x, 
                self.rect.y, 
                WHITE, 
                DrawTextureParams {
                    dest_size: Some(Vec2::new(self.rect.w, self.rect.h)),
                    ..Default::default()
                }
            );
        }
        
    }

    pub fn update(&mut self, mouse_pos: Vec2) {

        
        if self.rect.contains(
            mouse_pos
        ) {
            self.hovered = true;

            if is_mouse_button_down(macroquad::input::MouseButton::Left) {
                self.down = true;
            } else {
                self.down = false;
            }

            if is_mouse_button_released(macroquad::input::MouseButton::Left) {
                self.released = true;
            } else {
                self.released = false;
            }
        } else {
            self.hovered = false;
            self.down = false;
            self.released = false;
        }
    }
}