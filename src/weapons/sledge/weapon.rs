use std::{path::PathBuf, time::Duration};

use macroquad::math::Vec2;
use nalgebra::Vector2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapons::{lmg::{item::LMGItem, weapon_save::LMGSave}, weapon::weapon::Weapon, weapon_fire_context::WeaponFireContext}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct Sledge {
    pub weapon: Weapon 
}


impl Sledge {

    pub fn save(&self, space: &Space) -> LMGSave {
        LMGSave {
            weapon: self.weapon.save(space),
        }
    }

    pub fn fire(&mut self, ctx: &mut crate::ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        self.weapon.fire(ctx, weapon_fire_context, None, Some(1));
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader, facing: Facing) {
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
            weapon: Weapon::from_save(save.weapon, space, player_rigid_body_handle),
        }
    }
    

    pub fn to_item(&self, space: &Space) -> LMGItem {
        LMGItem {
            weapon: self.weapon.to_item(space),
        }
    }

    pub fn new(space: &mut Space, pos: Vector2<f32>, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {
        Self {
            weapon: Weapon::new(
                space,
                Default::default(),
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
            ),
        }
    }
}