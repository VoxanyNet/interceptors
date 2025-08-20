use std::path::{Path, PathBuf};

use macroquad::math::Vec2;
use nalgebra::Vector2;
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapon::{Weapon, WeaponFireContext}, ClientId, ClientTickContext};

pub struct Shotgun {
    weapon: Weapon
}

impl Shotgun {

    pub fn fire(&mut self, ctx: &mut ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        self.weapon.fire(ctx, weapon_fire_context, None, Some(1));
    }

    pub fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        self.weapon.player_joint_handle
    }

    pub fn reload(&mut self) {
        self.weapon.reload();
    }


    pub fn rigid_body_handle(&self) -> RigidBodyHandle {
        self.weapon.rigid_body
    }

    pub fn new(space: &mut Space, pos: Vector2<f32>, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {

        Self {
            weapon: Weapon::new(
                space, 
                pos, 
                owner, 
                player_rigid_body_handle, 
                PathBuf::from("assets\\shotgun.png"), 
                2., 
                Some(0.),
                Some(1.),
                PathBuf::from("assets\\sounds\\shotgun\\fire.wav"),
                20.,
                10.,
                0.,
                0.,
                None,
                Vec2::new(50., 11.),
                facing,
                web_time::Duration::from_millis(700),
                2,
                2,
                24

            ),
        }
        
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader, facing: Facing) {
        self.weapon.draw(
            space, 
            textures,
            facing
        ).await
    }
}