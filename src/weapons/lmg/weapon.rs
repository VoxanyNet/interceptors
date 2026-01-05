use std::{path::PathBuf, time::Duration};

use macroquad::math::Vec2;
use nalgebra::Vector2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapons::{lmg::{weapon_save::LMGSave}, weapon::weapon::WeaponBase, weapon_fire_context::WeaponFireContext}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct LMG {
    pub weapon: WeaponBase 
}


impl LMG {

    pub fn save(&self, space: &Space) -> LMGSave {
        LMGSave {
            weapon: self.weapon.save(space),
        }
    }

    pub fn mark_despawn(&mut self) {
        self.weapon.mark_despawn();
    }

    pub fn despawn_callback(&mut self, space: &mut Space) {
        self.weapon.despawn_callback(space);
    }

    pub fn fire(&mut self, ctx: &mut crate::ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        self.weapon.fire(ctx, weapon_fire_context, None, Some(1));
    }

    pub async fn draw(&self, space: &Space, textures: &TextureLoader, facing: Facing) {
        self.weapon.draw(
            space, 
            textures,
            facing
        ).await
    }

    pub fn reload(&mut self) {
        self.weapon.reload();
    }

    pub fn from_save(save: LMGSave, space: &mut Space, player_rigid_body_handle: Option<RigidBodyHandle>) -> Self {
        Self {
            weapon: WeaponBase::from_save(save.weapon, space, player_rigid_body_handle),
        }
    }
    

    pub fn new(space: &mut Space, pos: Vector2<f32>, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {
        Self {
            weapon: WeaponBase::new(
                owner,
                player_rigid_body_handle,
                PathBuf::from("assets\\lmg.png"),
                2.,
                None,
                Some(1.),
                PathBuf::from("assets\\sounds\\lmg_fire.wav"),
                10.,
                5.,
                0.,
                0.,
                None,
                Vec2::new(100., 32.),
                crate::player::Facing::Left,
                Duration::from_secs_f32(3.),
                10000,
                10000,
                100000,
                4.,
                50000.,
                web_time::Duration::from_millis(100),
                None,
                None

            ),
        }
    }
}