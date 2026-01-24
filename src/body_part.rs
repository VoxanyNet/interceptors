use std::path::PathBuf;

use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};

use crate::{Owner, draw_texture_onto_physics_body, space::Space, texture_loader::ClientTextureLoader};

#[derive(Debug, Clone)]
pub struct BodyPart {
    pub collider_handle: ColliderHandle,
    pub body_handle: RigidBodyHandle,
    sprite_path: PathBuf,
    scale: u16, 
    owner: Owner,
}

impl BodyPart {

    pub fn despawn(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }
    
    pub fn new(
        sprite_path: PathBuf,
        scale: u16,
        mass: f32,
        pos: glamx::Pose2,
        space: &mut Space,
        owner: Owner,
        texture_size: macroquad::math::Vec2
    ) -> Self {

        let rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .pose(
                    pos 
                )
                .ccd_enabled(true)
                .build()
        );

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(
                (texture_size.x / 2.) * scale as f32,
                (texture_size.y / 2.) * scale as f32
            )
                .mass(mass), 
            rigid_body_handle, 
            &mut space.rigid_body_set
        );

        Self {
            collider_handle,
            body_handle: rigid_body_handle,
            sprite_path: sprite_path,
            scale,
            owner,
        }

    }

    pub async fn draw(&self, textures: &ClientTextureLoader, space: &Space, flip_x: bool) {
        draw_texture_onto_physics_body(
            self.body_handle, 
            self.collider_handle, 
            &space, 
            &self.sprite_path, 
            textures, 
            flip_x, 
            false, 
            0.
        ).await
    }

}
