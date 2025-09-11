use std::path::PathBuf;

use macroquad::{color::Color, math::Vec2};
use nalgebra::Isometry2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{space::Space, texture_loader::TextureLoader, weapons::{shotgun::{item_save::ShotgunItemSave, weapon::Shotgun}, weapon::item::WeaponItem}, ClientId};

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
                base_damage: 20.,
                knockback: 100000.
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