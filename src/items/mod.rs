use downcast_rs::{Downcast, impl_downcast};
use macroquad::{color::Color, math::Vec2};
use rapier2d::prelude::ColliderHandle;

use crate::{Prefabs, TickContext, area::AreaContext, drawable::Drawable, items::item_save::ItemSave, player::PlayerContext, space::{self, Space}, texture_loader::ClientTextureLoader, weapons::{Weapon, ItemOwnerContext}};

pub mod prop;
pub mod item_save;

impl_downcast!(Item);

pub enum ConsumedStatus {
    Consumed,
    NotConsumed
}
pub trait Item: Downcast {

    
    fn stackable(&self) -> bool;
    fn save(&self, space: &Space) -> Box<dyn ItemSave>;
    fn draw_preview(
        &self, 
        textures: &ClientTextureLoader, 
        size: f32,
        draw_pos: Vec2,
        color: Option<Color>,
        rotation: f32
    );

    fn use_released(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus;

    fn use_hold(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus;


    fn same(&self, other: &dyn Item) -> bool;
    
    /// Downcast to weapon
    fn as_weapon(&self) -> Option<&dyn Weapon> {
        None
    }

    fn as_weapon_mut(&mut self) -> Option<&mut dyn Weapon> {
        None
    }
    fn get_preview_resolution(
        &self,
        textures: &ClientTextureLoader,
        size: f32
    ) -> Vec2;

    
    fn draw_active(&self, textures: &ClientTextureLoader);

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