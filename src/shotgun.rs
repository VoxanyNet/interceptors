use std::path::{Path, PathBuf};

use macroquad::{color::Color, math::Vec2};
use nalgebra::{Isometry2, Vector2};
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{player::Facing, space::Space, texture_loader::TextureLoader, weapon::{Weapon, WeaponFireContext, WeaponItem, WeaponItemSave, WeaponSave}, ClientId, ClientTickContext};


#[derive(PartialEq, Clone)]
pub struct Shotgun {
    weapon: Weapon
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShotgunSave {
    weapon: WeaponSave
}

#[derive(PartialEq, Clone, Debug)]
pub struct ShotgunItem {
    weapon: WeaponItem
}

impl ShotgunItem {

    pub fn new() -> Self {

        Self {
            weapon: WeaponItem {
                mass: 1.,
                texture_size: Vec2::new(50., 11.),
                sprite: PathBuf::from("assets\\shotgun.png"),
                scale: 2.,
                fire_sound_path: PathBuf::from("assets\\sounds\\shotgun\\fire.wav"),
                x_screen_shake_frequency: 20.,
                x_screen_shake_intensity: 10.,
                y_screen_shake_frequency: 0.,
                y_screen_shake_intensity: 0.,
                shell_sprite: None,
                rounds: 2,
                capacity: 2,
                reserve_capacity: 24,
                reload_duration: 0.7,
            },
        }
    }

    pub fn to_shotgun(
        &self, 
        space:&mut Space, 
        pos: Isometry2<f32>, 
        owner: ClientId, 
        player_rigid_body_handle: Option<RigidBodyHandle>
    ) -> Shotgun {
        Shotgun {
            weapon: self.weapon.into_weapon(space, pos, owner, player_rigid_body_handle),
        }
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

    pub fn save(&self) -> ShotgunItemSave {
        ShotgunItemSave {
            weapon: self.weapon.save(),
        }
    }

    pub fn from_save(save: ShotgunItemSave) -> Self {
        Self {
            weapon: WeaponItem::from_save(save.weapon),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShotgunItemSave {
    weapon: WeaponItemSave
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

    pub fn to_item(&self, space: &Space) -> ShotgunItem {
        ShotgunItem {
            weapon: self.weapon.to_item(space),
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