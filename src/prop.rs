use macroquad::math::Vec2;
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{draw_texture_onto_physics_body, space::Space, texture_loader::TextureLoader};

pub struct Prop {
    previous_position: Isometry2<f32>,
    rigid_body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle
}

#[derive(Serialize, Deserialize)]
pub struct PropPositionUpdate {
    pub position: Vec2,
    pub id: u32
}

impl Prop {

    pub fn server_tick(&mut self, space: &mut Space) {
        let position = space.rigid_body_set.get(self.rigid_body_handle).unwrap().position();

        if self.previous_position != *position {
            
        }
    }

    pub fn client_tick(&mut self, space: &mut Space) {
        
    }
    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {
        draw_texture_onto_physics_body(
            self.rigid_body_handle, 
            self.collider_handle, 
            space, 
            &"assets/brick_block.png".to_string(), 
            textures, 
            false, 
            false, 
            0.
        ).await
    }

    pub fn from_save(save: PropSave, space: &mut Space) -> Self {

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(vector![save.position.x, save.position.y].into())
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(save.width, save.height),
            body,
            &mut space.rigid_body_set
        );

        
        Self {
            rigid_body_handle: body,
            collider_handle: collider,
            previous_position: vector![save.position.x, save.position.y].into()
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PropSave {
    position: Vec2,
    width: f32,
    height: f32,
    sprite: String,
    id: u32
}