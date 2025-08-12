use std::time::Duration;

use macroquad::{math::Rect, ui::widgets::Texture};
use serde::{Deserialize, Serialize};

use crate::{area::{Area, AreaSave}, texture_loader::TextureLoader, ClientTickContext, ServerIO};

pub struct World {
    pub areas: Vec<Area>
}

impl World {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for area in &mut self.areas {
            area.client_tick(ctx);
        }
    }

    pub fn server_tick(&mut self, io:&mut ServerIO, dt: Duration) {

        for area in &mut self.areas {
            area.server_tick(io, dt);
        }
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect) {
    


        for area in &self.areas {
            area.draw(textures, camera_rect).await
        }
    } 
}