use std::path::PathBuf;

use macroquad::{color::WHITE, math::{vec2, Vec2}, texture::{draw_texture_ex, DrawTextureParams}};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::{parry::{bounding_volume::Aabb, shape::Cuboid}, prelude::{ColliderBuilder, ColliderHandle, Compound, RigidBody, RigidBodyBuilder, RigidBodyHandle, Shape, SharedShape}};

use crate::{rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, ClientTickContext};

pub struct CompoundTest {
    body: RigidBodyHandle,
    texture_path: PathBuf,
}

// COLLIDER FOr each pixel

// alpha mask? that is updated and drawn over the texture for each pixel that is removed

// texture BLENDING

impl CompoundTest {
    pub fn new(
        space: &mut Space, 
        ctx: &ClientTickContext, 
        sprite_path: PathBuf,
        x_scale: f32,
        y_scale: f32,
        pos: Vector2<f32>
    ) -> Self {

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos.into())
        );  
        
        let texture = ctx.textures.get(&sprite_path);

        let texture_data = texture.get_texture_data();

        // we need the half extents so we can offset the colliders so that the rigid body is still in the center
        let half_extents = Vec2::new(
         (texture.width() * x_scale) / 2., 
            (texture.height() * y_scale) / 2.
        );

        for x in 0..texture.width() as u32 {
            for y in 0..texture.height() as u32 {
                let color = texture_data.get_pixel(x, y);

                if color.a == 0. {
                    continue;
                }

                let translation = Vector2::new(
                (((x as f32 * x_scale)) - half_extents.x) + 0.5, 
                (((y as f32 * y_scale)) + half_extents.y) - 0.5    
                );

                let position = Isometry2::new(
                    translation, 
                    0.
                );

                let collider = ColliderBuilder::cuboid(
                    1., 
                    1.
                )
                    .position(position)
                    .mass(0.0001)
                    .build();

        
                space.collider_set.insert_with_parent(
                    collider, 
                    body, 
                    &mut space.rigid_body_set
                );
            }
        }


        Self {
            body: body,
            texture_path: sprite_path,
        }
    }

    pub fn draw(&self, space: &Space, textures: &TextureLoader) {
        
        let texture = textures.get(&self.texture_path);
        let rigid_body = space.rigid_body_set.get(self.body).unwrap();

        let macroquad_pos = rapier_to_macroquad(rigid_body.position().translation.vector);
        
        draw_texture_ex(
            texture, 
            macroquad_pos.x - (texture.width() / 2.), 
            macroquad_pos.y - (texture.height() / 2.), 
            WHITE, 
            DrawTextureParams {
                dest_size: None,
                source: None,
                rotation: rigid_body.rotation().angle() * -1.,
                flip_x: false,
                flip_y: false,
                pivot: None,
            }
        );
        
    }
}