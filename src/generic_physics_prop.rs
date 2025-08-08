use macroquad::math::Vec2;
use nalgebra::{vector, Isometry2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{draw_texture_onto_physics_body, space::Space, texture_loader::TextureLoader};

#[derive(Default)]
pub struct GenericPhysicsProp {
    rigid_body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    sprite_path: String,
    previous_pos: Isometry2<f32>
}

impl GenericPhysicsProp {

    pub fn set_pos(&mut self, position: Isometry2<f32>, space: &mut Space) {
        space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap().set_position(position, true);
    }
    
    pub fn from_save(save: GenericPhysicsPropSave, space: &mut Space) -> Self {

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(save.pos)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(save.size.x / 2., save.size.y / 2.)
                .mass(save.mass),
            body,
            &mut space.rigid_body_set
        );


        Self {
            rigid_body_handle: body,
            collider_handle: collider,
            sprite_path: save.sprite_path,
            previous_pos: save.pos,
        }
    }

    pub fn save(&self, space: &Space) -> GenericPhysicsPropSave {

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let pos = body.position().clone();
        
        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let mass = collider.mass();
        let size = collider.shape().as_cuboid().unwrap().half_extents;

        GenericPhysicsPropSave {
            size: Vec2::new(size.x, size.y),
            pos,
            mass,
            sprite_path: self.sprite_path.clone(),
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {
        draw_texture_onto_physics_body(
            self.rigid_body_handle, 
            self.collider_handle, 
            space, 
            &self.sprite_path, 
            textures, 
            false, 
            false, 
            0.
        ).await;

    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GenericPhysicsPropSave {
    pub size: Vec2,
    pub pos: Isometry2<f32>,
    pub mass: f32,
    pub sprite_path: String
}