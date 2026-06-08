use async_trait::async_trait;
use glamx::{Pose2,};
use macroquad::{color::Color, math::vec2, shapes::{DrawRectangleParams, draw_rectangle_ex}};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{DrawRectangleParameters, TickContext, drawable::{DrawContext, Drawable}, rapier_to_macroquad, space::Space, uuid_u64};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq)]
pub struct DissolvedPixelId {
    id: u64
}

impl DissolvedPixelId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}

pub struct DissolvedPixel {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
    color: Color,
    size: f32,
    spawned: web_time::Instant,
    pub despawn: bool
}


impl DissolvedPixel {

    pub fn tick(&mut self) {

        if self.despawn {
            return;
        }

        let elapsed = self.spawned.elapsed().as_secs_f32();
        
        if elapsed == 0. {
            return;
        }


        self.color.a -= 0.01 * elapsed;

        if self.color.a <= 0. {
            self.mark_despawn();
        }

    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }
    pub fn despawn_callback(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

    pub fn new(
        pos: Pose2, 
        space: &mut Space,
        color: Color,
        size: f32,
        mass: Option<f32>,
        velocity: Option<RigidBodyVelocity<f32>>,
    ) -> Self {

        let velocity = match velocity {
            Some(velocity) => velocity,
            None => RigidBodyVelocity::zero(),
        };

        let mass = match mass {
            Some(mass) => mass,
            None => 1.,
        };

        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos)
                .angvel(velocity.angvel)
                .linvel(velocity.linvel)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(size / 2., size / 2.).mass(mass),
            rigid_body,
            &mut space.rigid_body_set
        );

        Self {
            body: rigid_body,
            collider,
            color,
            size,
            spawned: web_time::Instant::now(),
            despawn: false
        }
    }

    pub fn draw(&self, ctx: &mut TickContext, space: &Space) {
        if self.despawn {
            return;
        }

        let body = space.rigid_body_set.get(self.body).unwrap();

        let macroquad_pos = rapier_to_macroquad(body.translation());

        let shape = space.collider_set.get(self.collider).unwrap().shape().as_cuboid().unwrap();

        ctx.add_draw_command(
            5, 
            DrawRectangleParameters {
                position: macroquad::math::vec2(macroquad_pos.x, macroquad_pos.y),
                size: macroquad::math::vec2(shape.half_extents.x * 2., shape.half_extents.y * 2.),
                offset: Some(macroquad::math::Vec2::new(0.5, 0.5)),
                rotation: Some(body.rotation().angle() * -1.),
                color: Some(self.color),
            }.into()
        );

    }
}

