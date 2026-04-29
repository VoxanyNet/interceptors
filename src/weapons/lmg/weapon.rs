use std::{path::PathBuf, time::Duration};

use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::RigidBodyHandle;
use delegate::delegate;

use crate::{ClientId, TickContext, area::AreaContext, items::{ConsumedStatus, Item, item_save::ItemSave}, player::{Facing, PlayerContext}, space::Space, texture_loader::ClientTextureLoader, weapons::{ItemOwnerContext, Weapon, lmg::weapon_save::LMGSave, weapon::weapon::{BaseWeapon, WeaponOwner}, weapon_fire_context::WeaponFireContext, weapon_type::ShooterContext}};

#[derive(PartialEq, Clone, Debug)]
pub struct LMG {
    pub weapon_base: BaseWeapon 
}

impl Item for LMG {
    delegate! {
        to self.weapon_base {
            fn as_weapon_mut(&mut self) -> Option<&mut dyn Weapon>;
            fn as_weapon(&self) -> Option<&dyn crate::weapons::Weapon>;
            fn use_released(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus;
            fn use_hold(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus;
            fn stackable(&self) -> bool;
            fn save(&self, space: &Space) -> Box<dyn ItemSave>;
            fn draw_preview(
                &self, 
                ctx: &mut TickContext, 
                size: f32,
                draw_pos: Vec2,
                color: Option<Color>,
                rotation: f32
            );
            fn get_preview_resolution(
                &self,
                textures: &ClientTextureLoader,
                size: f32
            ) -> Vec2;

            
            fn draw_active(&self, ctx: &mut TickContext, space: &Space);

            fn name(&self) -> String;

            fn equip(
                &mut self, 
                ctx: &mut TickContext, 
                area_context: &mut AreaContext, 
                player_context: &mut PlayerContext
            );

            fn unequip(
                &mut self, 
                ctx: &mut TickContext, 
                area_context: &mut AreaContext, 
                player_context: &mut PlayerContext
            );

            fn tick(
                &mut self,
                ctx: &mut TickContext, 
                area_context: &mut AreaContext, 
                player_context: &mut PlayerContext
            );
        }
    }

    fn same(&self, other: &dyn Item) -> bool {
        if let Some(other_concrete) = other.downcast_ref::<Self>() {
            other_concrete == self
        } else {
            false
        }
    }
}


impl LMG {
    

    pub fn new(_space: &mut Space, _pos: Vec2, owner: WeaponOwner, player_rigid_body_handle: Option<RigidBodyHandle>, _facing: Facing) -> Self {
        Self {
            weapon_base: BaseWeapon::new(
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
                web_time::Duration::from_millis(100),
                None,
                None

            ),
        }
    }
}