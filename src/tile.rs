use std::path::PathBuf;

use macroquad::{color::WHITE, math::Vec2, texture::{draw_texture_ex, DrawTextureParams}};
use nalgebra::{Isometry2, Vector2};
use rapier2d::prelude::{ColliderHandle, RigidBody, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{rapier_to_macroquad, space::Space, texture_loader::TextureLoader, uuid_u64};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq)]
pub struct TileId {
    id: u64
}

impl TileId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}
#[derive(PartialEq, Clone, Debug)]
pub struct Tile {
    sprite_path: PathBuf,
    pos: Vector2<f32>, // we store an additional position value because the tile doesnt always have a rigid body
    rigid_body_handle: Option<RigidBodyHandle>,
    collider_handle: Option<ColliderHandle>,
    id: TileId
}

impl Tile {
    pub fn new(position: Vector2<f32>, sprite_path: PathBuf) -> Self {
        Self {
            sprite_path,
            pos: position,
            rigid_body_handle: None,
            collider_handle: None,
            id: TileId::new(),
        }
    } 

    pub fn from_save(save: TileSave) -> Self {
        Self {
            sprite_path: save.sprite_path,
            pos: save.pos,
            rigid_body_handle: None,
            collider_handle: None,
            id: save.id,
        }
    }

    pub fn save(&self) -> TileSave {
        TileSave {
            sprite_path: self.sprite_path.clone(),
            pos: self.pos,
            id: self.id,
        }
    }

    pub fn draw(&self, textures: &TextureLoader) {

        let texture = textures.get(&self.sprite_path);

        let macroquad_pos = rapier_to_macroquad(self.pos);

        draw_texture_ex(
            texture, 
            macroquad_pos.x - 25., 
            macroquad_pos.y - 25., 
            WHITE, 
            DrawTextureParams {
                dest_size: Some(Vec2::new(50., 50.)),
                ..Default::default()
            }
        );

    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TileSave {
    sprite_path: PathBuf,
    pos: Vector2<f32>,
    id: TileId
}

