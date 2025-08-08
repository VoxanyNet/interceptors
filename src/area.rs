use macroquad::{color::WHITE, file::load_string, math::Vec2, texture::{draw_texture_ex, load_texture, DrawTextureParams}, window::get_internal_gl};
use macroquad_tiled::{load_map, Map};
use rapier2d::prelude::ColliderHandle;
use serde::{Deserialize, Serialize};

use crate::{prop::{Prop, PropSave}, space::Space, texture_loader::TextureLoader, ClientTickContext};

pub struct Clip {
    pub collider_handle: ColliderHandle
}

#[derive(Serialize, Deserialize)]
pub struct ClipSave {
    pub size: Vec2,
    pub pos: Vec2,
}

// equivalent to chunk in minecraft
pub struct Area {
    spawn_point: Vec2,
    space: Space
}

impl Area {
    pub fn empty() -> Self {

        
        Self {
            spawn_point: Vec2::ZERO,
            space: Space::new(),
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader) {
        
    }

    pub fn server_tick(&mut self) {

    }

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        
    }

    pub async fn from_save(save: AreaSave) -> Self {

        let mut space = Space::new();


        Self {
            spawn_point: save.spawn_point,
            space
        }
    }


}

#[derive(Serialize, Deserialize, Debug)]
pub struct AreaSave {
    spawn_point: Vec2,
    props: Vec<PropSave>,
    offset: Vec2,
    tile_map_path: String,
    tileset_data_path: String,
    tileset_texture_path: String
}