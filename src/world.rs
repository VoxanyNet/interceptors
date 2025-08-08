use macroquad::{math::Rect, ui::widgets::Texture};
use serde::{Deserialize, Serialize};

use crate::{area::{Area, AreaSave}, texture_loader::TextureLoader, ClientTickContext};

pub struct World {
    areas: Vec<Area>,
    pub lobby: Area
}

impl World {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for arena in &mut self.areas {
            arena.client_tick(ctx)
        }
    }

    pub fn server_tick(&mut self) {
           
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
            lobby: Area::empty()
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect) {
    

        self.lobby.draw(textures, camera_rect).await;

        for area in &self.areas {
            area.draw(textures, camera_rect).await
        }
    } 
}