use macroquad::ui::widgets::Texture;
use serde::{Deserialize, Serialize};

use crate::{area::{Area, AreaSave}, texture_loader::TextureLoader, ClientTickContext};

pub struct World {
    areas: Vec<Area>
}

impl World {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for arena in &mut self.areas {
            arena.client_tick(ctx)
        }
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader) {
        for area in &self.areas {
            area.draw(textures).await
        }
    } 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldSave {
    pub areas: Vec<AreaSave>
}