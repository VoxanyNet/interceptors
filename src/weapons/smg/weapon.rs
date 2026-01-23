use std::{path::PathBuf, str::FromStr};

use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};

use crate::{ClientId, ClientTickContext, TickContext, player::Facing, space::Space, texture_loader::ClientTextureLoader, weapons::{smg::weapon_save::SMGSave, weapon::weapon::WeaponBase, weapon_fire_context::WeaponFireContext}};

#[derive(PartialEq, Clone, Debug)]
pub struct SMG {
    pub weapon_base: WeaponBase
}

impl SMG {

    pub async fn draw(&self, space: &Space, textures: &ClientTextureLoader, facing: Facing) {
        self.weapon_base.draw(space, textures, facing).await
    }
    pub fn mark_despawn(&mut self) {
        self.weapon_base.mark_despawn();
    }

    pub fn despawn_callback(&mut self, space: &mut Space) {
        self.weapon_base.despawn_callback(space);
    }

    pub fn preview_name(&self) -> String {
        "SMG".to_string()
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &ClientTextureLoader) -> Vec2 {
        self.weapon_base.get_preview_resolution(size, textures)
    }

    pub fn draw_preview(&self, textures: &ClientTextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        self.weapon_base.draw_preview(textures, size, draw_pos, color, rotation);
    }

    pub fn save(&self, space: &Space) -> SMGSave {
        SMGSave {
            weapon_base: self.weapon_base.save(space),
        }
    }

    pub fn from_save(save: SMGSave, space: &mut Space, player_rigid_body_handle: Option<RigidBodyHandle>) -> Self {
        Self {
            weapon_base: WeaponBase::from_save(save.weapon_base , space, player_rigid_body_handle),
        }
    }

    pub fn fire(&mut self, ctx: &mut TickContext, weapon_fire_context: &mut WeaponFireContext) {
        
        
        self.weapon_base.fire(
            ctx, 
            weapon_fire_context, 
            Some(0.1), 
            Some(1)
        );
    }

    pub fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        self.weapon_base.player_joint_handle
    }

    pub fn reload(&mut self) {
        self.weapon_base.reload();
    }

    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        self.weapon_base.rigid_body
    }


    pub fn new(owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {

        Self {
            weapon_base: WeaponBase::new(
                owner, 
                player_rigid_body_handle, 
                PathBuf::from("assets\\smg.png"), 
                2., 
                Some(0.),
                Some(1.),
                PathBuf::from("assets\\sounds\\smg\\smg_middle.wav"),
                20.,
                10.,
                0.,
                0.,
                None,
                Vec2::new(26., 16.),
                facing,
                web_time::Duration::from_millis(700),
                200,
                200,
                240,
                10.,
                100000.,
                web_time::Duration::from_millis(50),
                Some(PathBuf::from_str("assets\\sounds\\smg\\smg_start.wav").unwrap()),
                Some(PathBuf::from_str("assets\\sounds\\smg\\smg_end.wav").unwrap())
            ),
        }
    }




}