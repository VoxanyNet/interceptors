use nalgebra::{Isometry2, Vector2};
use rapier2d::prelude::ColliderHandle;

#[derive(Clone)]
pub struct BulletImpactData {
    pub shooter_pos: Isometry2<f32>,
    pub impacted_collider: ColliderHandle,
    pub bullet_vector: Vector2<f32>,
    pub damage: f32,
    pub knockback: f32
} 