use std::path::PathBuf;

use macroquad::math::Rect;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::space::Space;

pub struct Fragment {
    texture: PathBuf,
    texture_source: Rect,
    body: RigidBodyHandle,
    collider: ColliderHandle,
    scale: f32
}

impl Fragment {
    pub fn new(_space: &mut Space, _texture: PathBuf, _texture_source: Rect, _mass: f32, _scale: f32) {
        
    }
}