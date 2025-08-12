use macroquad::math::Vec2;
use nalgebra::{vector, Isometry2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};

use crate::{space::Space, texture_loader::TextureLoader, ClientId, ClientTickContext};

pub struct BodyPart {
    collider_handle: ColliderHandle,
    body_handle: RigidBodyHandle,
    sprite_path: String,
    scale: u16, 
    owner: ClientId,
    previous_pos: Isometry2<f32>
}

impl BodyPart {
    pub fn new(
        sprite_path: String,
        scale: u16,
        mass: f32,
        pos: Vec2,
        space: &mut Space,
        textures: &mut TextureLoader,
        owner: ClientId,
        texture_size: Vec2
    ) -> Self {

        let rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(vector![pos.x, pos.y].into())
                .ccd_enabled(true)
                .build()
        );

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(
                (texture_size.x / 2.) * scale as f32, 
                (texture_size.y / 2.) * scale as f32
            ), 
            rigid_body_handle, 
            &mut space.rigid_body_set
        );

        Self {
            collider_handle,
            body_handle: rigid_body_handle,
            sprite_path,
            scale,
            owner,
        }

    }

    pub fn owner_tick(&mut self, ctx: &mut ClientTickContext, space: &mut Space) {
        
        let current_pos = space.rigid_body_set.get(self.body_handle).unwrap().position();

        if *self.previous_pos != current_pos {
            
        }
    }

    pub fn tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        if *ctx.client_id != self.owner {
            self.owner_tick(ctx);
        }


    }
}