use std::path::PathBuf;

use macroquad::math::Rect;
use rapier2d::prelude::{ColliderHandle, RigidBody, RigidBodyHandle};

use crate::space::Space;

pub struct Fragment {
    texture: PathBuf,
    texture_source: Rect,
    body: RigidBodyHandle,
    collider: ColliderHandle,
    scale: f32
}

impl Fragment {
    pub fn new(space: &mut Space, texture: PathBuf, texture_source: Rect, mass: f32, scale: f32) {
        
    }
}