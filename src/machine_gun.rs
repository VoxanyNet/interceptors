use std::{path::PathBuf, time::Duration};

use macroquad::math::Vec2;
use nalgebra::Vector2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{player::Facing, space::Space, weapon::Weapon, ClientId};

pub struct LMG {
    weapon: Weapon 
}

impl LMG {
    pub fn new(space: &mut Space, pos: Vector2<f32>, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>, facing: Facing) -> Self {
        Self {
            weapon: Weapon::new(
                space,
                Default::default(),
                owner,
                player_rigid_body_handle,
                PathBuf::from("assets\\lmg.png"),
                2.,
                None,
                Some(1.),
                PathBuf::from("assets\\sounds\\lmg_fire.wav"),
                10.,
                5.,
                0.,
                0.,
                None,
                Vec2::new(100., 32.),
                crate::player::Facing::Left,
                Duration::from_secs_f32(3.),
                10000,
                10000,
                100000,
                4.,
                50000.,
            ),
        }
    }
}