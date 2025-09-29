use std::path::PathBuf;

use macroquad::{color::Color, math::Vec2};
use nalgebra::Vector2;
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapons::{shotgun::{ weapon_save::ShotgunSave}, weapon::weapon::Weapon, weapon_fire_context::WeaponFireContext}, ClientId, ClientTickContext};

#[derive(PartialEq, Clone, Debug)]
pub struct Shotgun {
    pub weapon: Weapon
}

impl Shotgun {

    pub fn despawn(&mut self, space: &mut Space) {
        self.weapon.despawn(space);
    }
    pub fn preview_name(&self) -> String {
        "Shotgun".to_string()
    }
    
    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        self.weapon.get_preview_resolution(size, textures)
    }

    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        self.weapon.draw_preview(textures, size, draw_pos, color, rotation);
    }
    pub fn save(&self, space: &Space) -> ShotgunSave {
        ShotgunSave {
            weapon: self.weapon.save(space),
        }
    }

    pub fn from_save(save: ShotgunSave, space: &mut Space, player_rigid_body_handle: Option<RigidBodyHandle>) -> Self {
        Self {
            weapon: Weapon::from_save(save.weapon, space, player_rigid_body_handle),
        }
    }
    pub fn fire(&mut self, ctx: &mut ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        self.weapon.fire(ctx, weapon_fire_context, None, Some(1));
    }

    pub fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        self.weapon.player_joint_handle
    }

    pub fn reload(&mut self) {
        self.weapon.reload();
    }


    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        self.weapon.rigid_body
    }

    pub fn new(owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {

        Self {
            weapon: Weapon::new(
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
                24,
                100.,
                100000.
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