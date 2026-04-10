use std::path::PathBuf;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::{ClientId, items::{Item, item_save::ItemSave}, player::Facing, weapons::weapon::weapon::{BaseWeapon, WeaponOwner}};

// maybe this isnt the best idea to save all this info explicitly and just have the specific weapon types handle saving but idk this seems like it will save some time
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WeaponSave {
    pub mass: f32,
    pub texture_size: Vec2,
    pub sprite: PathBuf,
    pub owner: WeaponOwner,
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
    pub knockback: f32,
    pub fire_cooldown: web_time::Duration,
    pub hold_fire_begin_sound_path: Option<PathBuf>,
    pub hold_fire_end_sound_path: Option<PathBuf>

}

#[typetag::serde]
impl ItemSave for WeaponSave {
    fn load(&self) -> Box<dyn Item>  {
        Box::new(
            BaseWeapon::new(
                self.owner.clone(), 
                None, // GOING TO BE AN ISSUE PROBABLY
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
                Facing::Right, // this parameter doesnt do anything in new()
                web_time::Duration::from_secs_f32(self.reload_duration), 
                self.rounds, 
                self.capacity, 
                self.reserve_capacity,
                self.base_damage,
                self.knockback,
                self.fire_cooldown,
                self.hold_fire_begin_sound_path.clone(),
                self.hold_fire_end_sound_path.clone()
            )
        )
    }

}