use macroquad::math::Vec2;
use nalgebra::vector;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle};
use serde::{Deserialize, Serialize};

use crate::space::Space;


pub struct Clip {
    pub collider_handle: ColliderHandle
}

impl Clip {
    pub fn from_save(save: ClipSave, space: &mut Space) -> Self {

        let collider_handle = space.collider_set.insert(
            ColliderBuilder::cuboid(
                save.size.x / 2., 
                save.size.y / 2.
            )
                .position(vector![save.pos.x, save.pos.y].into())
        );

        Self {
            collider_handle,
        }
    }

    pub fn save(&self, space: &Space) -> ClipSave {

        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let shape = collider.shape().as_cuboid().unwrap();

        ClipSave {
            size: Vec2::new(shape.half_extents.x * 2., shape.half_extents.y * 2.),
            pos: Vec2::new(collider.position().translation.x, collider.translation().y),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClipSave {
    pub size: Vec2,
    pub pos: Vec2,
}