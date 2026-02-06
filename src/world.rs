
use macroquad::{camera::Camera2D, math::Rect};

use crate::{Prefabs, ServerIO, TickContext, area::Area, font_loader::FontLoader, texture_loader::ClientTextureLoader};

pub struct World {
    pub areas: Vec<Area>
}

impl World {
    pub fn tick(&mut self, ctx: &mut TickContext) {
        for area in &mut self.areas {
            area.tick(ctx);
        }
    }

    pub fn server_tick(&mut self, io: &mut ServerIO, dt: web_time::Duration) {

        for area in &mut self.areas {
            area.server_tick(io, dt);
        }
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
        }
    }

    pub fn draw_hud(&self, textures: &ClientTextureLoader) {
        for area in &self.areas {
            area.draw_hud(textures);
        }
    }

    pub async fn draw(
        &mut self, 
        textures: &mut ClientTextureLoader, 
        camera_rect: &Rect, 
        prefabs: &Prefabs, 
        camera: &Camera2D, 
        fonts: &FontLoader, 
        elapsed: web_time::Duration
    ) {

        for area in &mut self.areas {
            area.draw(
                textures, 
                camera_rect, 
                prefabs, 
                camera, 
                fonts, 
                elapsed, 
                vec![], 
                false
            ).await
        }
    } 
}