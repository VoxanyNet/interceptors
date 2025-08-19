use std::path::PathBuf;

use macroquad::math::Vec2;
use nalgebra::Isometry2;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};

use crate::{draw_texture_onto_physics_body, space::Space, texture_loader::TextureLoader, ClientId, ClientTickContext};

pub struct BodyPart {
    pub collider_handle: ColliderHandle,
    pub body_handle: RigidBodyHandle,
    sprite_path: PathBuf,
    scale: u16, 
    owner: ClientId,
}

impl BodyPart {
    pub fn new(
        sprite_path: PathBuf,
        scale: u16,
        mass: f32,
        pos: Isometry2<f32>,
        space: &mut Space,
        owner: ClientId,
        texture_size: Vec2
    ) -> Self {

        let rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos)
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

    pub async fn draw(&self, textures: &mut TextureLoader, space: &Space, flip_x: bool) {
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

    pub fn owner_tick(&mut self, ctx: &mut ClientTickContext, space: &mut Space) {
        
    }

    pub fn tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        if *ctx.client_id != self.owner {
            self.owner_tick(ctx, space);
        }


    }
}
