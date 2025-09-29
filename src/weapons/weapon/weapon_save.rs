use std::path::PathBuf;

use macroquad::math::Vec2;
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

use crate::ClientId;

// maybe this isnt the best idea to save all this info explicitly and just have the specific weapon types handle saving but idk this seems like it will save some time
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WeaponSave {
    pub mass: f32,
    pub texture_size: Vec2,
    pub sprite: PathBuf,
    pub owner: ClientId,
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
    pub reload_duration: f32, // reload duration time in seconds
    pub base_damage: f32,
    pub knockback: f32

}