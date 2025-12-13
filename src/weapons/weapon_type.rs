use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::{ColliderHandle, ImpulseJointHandle, RigidBodyHandle};

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapons::{lmg::weapon::LMG, shotgun::weapon::Shotgun, weapon_fire_context::WeaponFireContext, weapon_type_save::WeaponTypeSave}, ClientTickContext};

#[derive(PartialEq, Clone, Debug)]
pub enum WeaponType {
    Shotgun(Shotgun),
    LMG(LMG)
}

impl WeaponType {


    pub fn stackable(&self) -> bool {
        false
    }
    pub fn unequip(&mut self, space: &mut Space) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.weapon.unequip(space),
            WeaponType::LMG(lmg) => lmg.weapon.unequip(space),
        }
    }
    pub fn equip(&mut self, space: &mut Space, player_rigid_body_handle: RigidBodyHandle) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.weapon.equip(space, player_rigid_body_handle),
            WeaponType::LMG(lmg) => lmg.weapon.equip(space, player_rigid_body_handle),
        }
    }
    pub fn name(&self) -> String {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.preview_name(),
            WeaponType::LMG(lmg) => "LMG".to_string()
        }
    }


    pub fn collider_handle(&self) -> Option<ColliderHandle> {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.weapon.collider,
            WeaponType::LMG(lmg)  => lmg.weapon.collider
        }
    }

    pub fn despawn(&mut self, space: &mut Space) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.despawn(space),
            WeaponType::LMG(lmg) => lmg.weapon.despawn(space),
        }
    }

    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.draw_preview(textures, size, draw_pos, color, rotation),
            WeaponType::LMG(lmg) => {}
        }
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.get_preview_resolution(size, textures),
            WeaponType::LMG(lmg) => Vec2::ZERO 
        }
    }
    pub fn save(&self, space: &Space) -> WeaponTypeSave {
        return match self {
            WeaponType::Shotgun(shotgun) => WeaponTypeSave::Shotgun(shotgun.save(space)),
            WeaponType::LMG(lmg) => WeaponTypeSave::LMG(lmg.save(space))
        }
    }

    pub fn from_save(space: &mut Space, save: WeaponTypeSave, player_rigid_body_handle:Option<RigidBodyHandle> ) -> Self {
        return match save {
            WeaponTypeSave::Shotgun(shotgun_save) => WeaponType::Shotgun(Shotgun::from_save(shotgun_save, space, player_rigid_body_handle)),
            WeaponTypeSave::LMG(lmg_save) => WeaponType::LMG(LMG::from_save(lmg_save, space, player_rigid_body_handle))
        }
    }

    pub fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.player_joint_handle(),
            WeaponType::LMG(lmg) => lmg.weapon.player_joint_handle
        }
    }

    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.rigid_body_handle(),
            WeaponType::LMG(lmg) => lmg.weapon.rigid_body
        }
    }
    pub fn fire(&mut self, ctx: &mut ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.fire(ctx, weapon_fire_context),
            WeaponType::LMG(lmg) => lmg.fire(ctx, weapon_fire_context)
        }
    }

    pub fn reload(&mut self) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.reload(),
            WeaponType::LMG(lmg) => lmg.reload(),
        }
    }

    pub async fn draw(&self, space: &Space, textures: &TextureLoader, facing: Facing) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.draw(space, textures, facing).await,
            WeaponType::LMG(lmg) => lmg.draw(space, textures, facing).await
        }
    }

}

