use rapier2d::prelude::ColliderHandle;

#[derive(Clone)]
pub struct BulletImpactData {
    pub shooter_pos: glamx::Vec2,
    pub impacted_collider: ColliderHandle,
    pub bullet_vector: glamx::Vec2,
    pub damage: f32,
    pub knockback: f32
} 