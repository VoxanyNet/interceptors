use macroquad::math::Vec2;
use nalgebra::vector;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::space::Space;


pub struct Clip {
    pub collider_handle: ColliderHandle,
    pub rigid_body_handle: RigidBodyHandle
}

impl Clip {
    pub fn from_save(save: ClipSave, space: &mut Space) -> Self {

        let rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::fixed()
                .position(vector![save.pos.x, save.pos.y].into())
        );

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(
                save.size.x / 2., 
                save.size.y / 2.
            ),
            rigid_body_handle,
            &mut space.rigid_body_set
        );

        Self {
            rigid_body_handle,
            collider_handle,
        }
    }

    pub fn save(&self, space: &Space) -> ClipSave {

        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let shape = collider.shape().as_cuboid().unwrap();

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let position = body.position();
        ClipSave {
            size: Vec2::new(shape.half_extents.x * 2., shape.half_extents.y * 2.),
            pos: Vec2::new(position.translation.x, position.translation.y),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClipSave {
    pub size: Vec2,
    pub pos: Vec2,
}