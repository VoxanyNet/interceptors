use std::{path::PathBuf, str::FromStr};

use async_trait::async_trait;
use delegate::delegate;
use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::{ImpulseJointHandle, RigidBodyHandle};

use crate::{ClientId, TickContext, area::AreaContext, drawable::{DrawContext, Drawable}, items::{ConsumedStatus, Item, item_save::ItemSave}, player::{Facing, PlayerContext}, space::Space, texture_loader::ClientTextureLoader, weapons::{ItemOwnerContext, smg::weapon_save::SMGSave, weapon::weapon::{BaseWeapon, WeaponOwner}, weapon_fire_context::WeaponFireContext, weapon_type::ShooterContext}};

#[derive(PartialEq, Clone, Debug)]
pub struct SMG {
    pub weapon_base: BaseWeapon
}

impl SMG {

    pub fn new(
        owner: WeaponOwner, 
        player_rigid_body_handle: Option<RigidBodyHandle>, 
        facing: Facing
    ) -> Self {

        Self {
            weapon_base: BaseWeapon::new(
                owner, 
                player_rigid_body_handle, 
                PathBuf::from("assets\\smg.png"), 
                2., 
                Some(0.),
                Some(1.),
                PathBuf::from("assets\\sounds\\smg\\smg_middle.wav"),
                20.,
                10.,
                0.,
                0.,
                None,
                Vec2::new(26., 16.),
                facing,
                web_time::Duration::from_millis(700),
                200,
                200,
                240,
                10.,
                100000.,
                web_time::Duration::from_millis(50),
                Some(PathBuf::from_str("assets\\sounds\\smg\\smg_start.wav").unwrap()),
                Some(PathBuf::from_str("assets\\sounds\\smg\\smg_end.wav").unwrap())
            ),
        }
    }

}

#[async_trait]
impl Drawable for SMG {
    async fn draw(&mut self, draw_context: &DrawContext) {
        todo!()
    }

    fn draw_layer(&self) -> u32 {
        self.weapon_base.draw_layer()
    }
}   

impl Item for SMG {

    fn same(&self, other: &dyn Item) -> bool {
        if let Some(other_concrete) = other.downcast_ref::<Self>() {
            other_concrete == self
        } else {
            false
        }
    }
    delegate! {
        to self.weapon_base {
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
}