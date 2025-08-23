
use macroquad::{camera::Camera2D, math::Rect, prelude::camera::mouse::Camera};

use crate::{area::Area, font_loader::FontLoader, texture_loader::TextureLoader, ClientTickContext, Prefabs, ServerIO};

pub struct World {
    pub areas: Vec<Area>
}

impl World {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for area in &mut self.areas {
            area.client_tick(ctx);
        }
    }

    pub fn server_tick(&mut self, io:&mut ServerIO, dt: web_time::Duration) {

        for area in &mut self.areas {
            area.server_tick(io, dt);
        }
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
        }
    }


    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect, prefabs: &Prefabs, camera: &Camera2D, fonts: &FontLoader) {
    


        for area in &self.areas {
            area.draw(textures, camera_rect, prefabs, camera, fonts).await
        }
    } 
}