use std::{path::PathBuf, time::Duration};

use macroquad::{color::Color, math::Vec2};
use nalgebra::Isometry2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{space::Space, texture_loader::TextureLoader, weapons::{lmg::{item_save::LMGItemSave, weapon::LMG}, weapon::item::WeaponItem}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct LMGItem {
    pub weapon: WeaponItem
}

impl LMGItem {

    pub fn new() -> Self {
        Self {
            weapon: WeaponItem {
                mass: 1.,
                texture_size: Vec2::new(100., 32.),
                sprite: PathBuf::from("assets\\lmg.png"),
                scale: 0.75,
                fire_sound_path: PathBuf::from("assets\\sounds\\lmg_fire.wav"),
                x_screen_shake_frequency: 10.,
                x_screen_shake_intensity: 5.,
                y_screen_shake_frequency: 0.,
                y_screen_shake_intensity: 0.,
                shell_sprite: None,
                rounds: 10000,
                capacity: 10000,
                reserve_capacity: 10000000,
                reload_duration: Duration::from_secs_f32(3.).as_secs_f32(),
                base_damage: 1.,
                knockback: 50000.,
            },
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        self.weapon.draw_preview(textures, size, draw_pos, color, rotation);
    }    

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        self.weapon.get_preview_resolution(size, textures)
    }

    pub fn save(&self) -> LMGItemSave {
        LMGItemSave {
            weapon: self.weapon.save(),
        }
    }

    pub fn from_save(save: LMGItemSave) -> Self {
        Self {
            weapon: WeaponItem::from_save(save.weapon),
        }
    }

    pub fn to_lmg(&self, 
        space:&mut Space, 
        pos: Isometry2<f32>, 
        owner: ClientId, 
        player_rigid_body_handle: Option<RigidBodyHandle>
    ) -> LMG {
        LMG {
            weapon: self.weapon.into_weapon(space, pos, owner, player_rigid_body_handle),
        }
    }

}




