
use macroquad::{camera::Camera2D, math::Rect};

use crate::{ClientId, ClientTickContext, Prefabs, ServerIO, TickContext, area::Area, font_loader::FontLoader, material_loader::MaterialLoader, texture_loader::ClientTextureLoader};

pub struct World {
    pub areas: Vec<Area>
}

impl World {
    pub fn tick(&mut self, ctx: &mut TickContext) {
        for area in &mut self.areas {
            area.tick(ctx);
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

    pub fn draw(
        &mut self, 
        ctx: &mut TickContext
    ) {
        
        for area in &mut self.areas {
            area.draw(
                ctx
            )
        }
    } 
}