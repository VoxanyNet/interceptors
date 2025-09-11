use std::path::PathBuf;

use macroquad::{color::Color, math::Vec2};
use nalgebra::Isometry2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{draw_preview, get_preview_resolution, player::Facing, space::Space, texture_loader::TextureLoader, weapons::weapon::{item_save::WeaponItemSave, weapon::Weapon}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct WeaponItem {
    pub mass: f32,
    pub texture_size: Vec2,
    pub sprite: PathBuf,
    pub scale: f32,
    pub fire_sound_path: PathBuf,
    pub x_screen_shake_frequency: f64,
    pub x_screen_shake_intensity: f64,
    pub y_screen_shake_frequency: f64,
    pub y_screen_shake_intensity: f64,
    pub shell_sprite: Option<String>,
    pub rounds: u32,
    pub capacity: u32,
    pub reserve_capacity: u32,
    pub reload_duration: f32,
    pub base_damage: f32,
    pub knockback: f32
}


impl WeaponItem {


    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        draw_preview(textures, size, draw_pos, color, rotation, &self.sprite);
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        get_preview_resolution(size, textures, &self.sprite)
    }

    pub fn from_save(save: WeaponItemSave) -> Self {

        Self {
            mass: save.mass,
            texture_size: save.texture_size,
            sprite: save.sprite,
            scale: save.scale,
            fire_sound_path: save.fire_sound_path,
            x_screen_shake_frequency: save.x_screen_shake_frequency,
            x_screen_shake_intensity: save.x_screen_shake_intensity,
            y_screen_shake_frequency: save.y_screen_shake_frequency,
            y_screen_shake_intensity: save.y_screen_shake_intensity,
            shell_sprite: save.shell_sprite,
            rounds: save.rounds,
            capacity: save.capacity,
            reserve_capacity: save.reserve_capacity,
            reload_duration: save.reload_duration,
            base_damage: save.base_damage,
            knockback: save.knockback
        }
    }

    pub fn save(&self) -> WeaponItemSave {
        WeaponItemSave {
            mass: self.mass,
            texture_size: self.texture_size,
            sprite: self.sprite.clone(),
            scale: self.scale,
            fire_sound_path: self.fire_sound_path.clone(),
            x_screen_shake_frequency: self.x_screen_shake_frequency,
            x_screen_shake_intensity: self.x_screen_shake_intensity,
            y_screen_shake_frequency: self.y_screen_shake_frequency,
            y_screen_shake_intensity: self.y_screen_shake_intensity,
            shell_sprite: self.shell_sprite.clone(),
            rounds: self.rounds,
            capacity: self.capacity,
            reserve_capacity: self.reserve_capacity,
            reload_duration: self.reload_duration,
            base_damage: self.base_damage,
            knockback: self.knockback
        }
    }
    pub fn into_weapon(
        &self, 
        space: &mut Space, 
        pos: Isometry2<f32>, 
        owner: ClientId,
        player_rigid_body_handle: Option<RigidBodyHandle>
    ) -> Weapon {
        Weapon::new(
            space,
            pos.translation.vector,
            owner,
            player_rigid_body_handle,
            self.sprite.clone(),
            self.scale,
            None,
            Some(self.mass),
            self.fire_sound_path.clone(),
            self.x_screen_shake_frequency,
            self.x_screen_shake_intensity,
            self.y_screen_shake_frequency,
            self.y_screen_shake_intensity,
            self.shell_sprite.clone(),
            self.texture_size,
            Facing::Right,
            web_time::Duration::from_secs_f32(self.reload_duration),
            self.rounds,
            self.capacity,
            self.reserve_capacity,
            self.base_damage,
            self.knockback
        )
    }
}