use std::path::PathBuf;

use macroquad::{color::{GRAY, WHITE}, math::Vec2, texture::{draw_texture_ex, DrawTextureParams}};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBody, RigidBodyBuilder, RigidBodyHandle};
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
    rigid_body_handle: Option<RigidBodyHandle>,
    collider_handle: Option<ColliderHandle>,
}

impl Tile {

    pub fn materialize(&mut self, tile_index: Vector2<usize>, space: &mut Space) {

        if self.rigid_body_handle.is_some() {
            return;
        }

    
        self.rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::fixed()
                .position(vector![
                    (tile_index.x as f32 * 50.),
                    (tile_index.y as f32 * 50.)
                ].into())
        ).into();

        self.collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::round_cuboid(25., 25., 0.5), 
            self.rigid_body_handle.unwrap(), 
            &mut space.rigid_body_set
        ).into();

    }
    pub fn new(sprite_path: PathBuf) -> Self {
        Self {
            sprite_path,
            rigid_body_handle: None,
            collider_handle: None,
        }
    } 

    pub fn from_save(save: TileSave) -> Self {
        Self {
            sprite_path: save.sprite_path,
            rigid_body_handle: None,
            collider_handle: None,
        }
    }

    pub fn save(&self) -> TileSave {
        
        TileSave {
            sprite_path: self.sprite_path.clone()
        }
    }

    pub fn draw(&self, textures: &TextureLoader, position: Vector2<usize>) {

        let texture = textures.get(&self.sprite_path);

        let macroquad_pos = rapier_to_macroquad(Vector2::new(position.x as f32, position.y as f32));

        let color = match self.rigid_body_handle {
            Some(_) => GRAY,
            None => WHITE,
        };

        draw_texture_ex(
            texture, 
            macroquad_pos.x - 25., 
            macroquad_pos.y - 25., 
            color, 
            DrawTextureParams {
                dest_size: Some(Vec2::new(50., 50.)),
                ..Default::default()
            }
        );

    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TileSave {
    pub sprite_path: PathBuf,
}

