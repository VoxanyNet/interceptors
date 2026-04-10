use std::path::PathBuf;

use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};

use crate::{ClientId, TickContext, area::AreaContext, player::{Facing, PlayerContext}, space::Space, texture_loader::ClientTextureLoader, weapons::{shotgun::weapon_save::ShotgunSave, weapon::weapon::{BaseWeapon, WeaponOwner}, weapon_fire_context::WeaponFireContext, weapon_type::ShooterContext}};

#[derive(PartialEq, Clone, Debug)]
pub struct Shotgun {
    pub weapon: BaseWeapon
}

impl Shotgun {

    pub fn new(
        owner: WeaponOwner, 
        player_rigid_body_handle: Option<RigidBodyHandle>, 
        facing: Facing
    ) -> Self {

        Self {
            weapon: BaseWeapon::new(
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
                100000.,
                web_time::Duration::from_secs(1),
                None,
                None
            ),
        }
        
    }

}