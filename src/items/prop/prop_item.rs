use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{TickContext, draw_preview, drawable::{DrawContext, Drawable}, get_preview_resolution, items::{ConsumedStatus, Item, item_save::ItemSave}, prop::Prop, props::wooden_box::wooden_box::WoodenBox, weapons::ItemOwnerContext};


// Props: The actual prop in the game world
// Prop save: Game world prop serialized
// Prop item: Simple enum that spawns in corresponding game world prop
// Prop item save: literally the same type as prop item because we dont need a seperate type for serializing because its so simple
// Weapons are different because they dont have a seperate item type because they implement Item directly. This requires more code
// Not sure how we should handle more complex prop items that require state in their item form
// Technically we could have different item types for each simple prop but this simplifies things a bit
// Maybe just have dedicated types for more complex prop types

/// This name isn't great but I want to make it clear that not every prop needs to be in this enum
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum SimplePropItem {
    WoodenBox
}

impl SimplePropItem {
    fn get_preview_texture(&self) -> PathBuf {
        match self {
            SimplePropItem::WoodenBox => {
                PathBuf::from_str("assets/box2.png").unwrap()
            },
        }
    }
}


impl Item for SimplePropItem {
    fn stackable(&self) -> bool {
        true
    }

    fn save(&self, space: &crate::space::Space) -> Box<dyn crate::items::item_save::ItemSave> {
        Box::new(self.clone())
    }

    fn use_hold(&mut self, ctx: &mut crate::TickContext, area_context: &mut crate::area::AreaContext,  weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus {
        todo!()
    }

    fn use_released(&mut self, ctx: &mut crate::TickContext, area_context: &mut crate::area::AreaContext,  weapon_owner_context: &mut ItemOwnerContext) -> ConsumedStatus {
        todo!()
    }

    

    fn draw_preview(
        &self, 
        ctx: &mut TickContext,
        size: f32,
        draw_pos: macroquad::prelude::Vec2,
        color: Option<macroquad::prelude::Color>,
        rotation: f32
    ) {

        let texture_path = &self.get_preview_texture();
        draw_preview(ctx, size, draw_pos, color, rotation, texture_path, 1);
    }

    fn same(&self, other: &dyn Item) -> bool {
        if let Some(other_concrete) = other.downcast_ref::<Self>() {
            other_concrete == self
        } else {
            false
        }
    }


    fn get_preview_resolution(
        &self,
        textures: &crate::texture_loader::ClientTextureLoader,
        size: f32
    ) -> macroquad::prelude::Vec2 {

        let preview_texture_path = self.get_preview_texture();
        
        get_preview_resolution(size, textures, &preview_texture_path)
    }

    fn draw_active(&self, ctx: &mut TickContext, space: &crate::space::Space) {
        todo!()
    }

    fn name(&self) -> String {
        todo!()
    }

    fn equip(
        &mut self, 
        ctx: &mut crate::TickContext, 
        area_context: &mut crate::area::AreaContext, 
        player_context: &mut crate::player::PlayerContext
    ) {
        todo!()
    }

    fn unequip(
        &mut self, 
        ctx: &mut crate::TickContext, 
        area_context: &mut crate::area::AreaContext, 
        player_context: &mut crate::player::PlayerContext
    ) {
        todo!()
    }

    fn tick(
        &mut self,
        ctx: &mut crate::TickContext, 
        area_context: &mut crate::area::AreaContext, 
        player_context: &mut crate::player::PlayerContext
    ) {
        todo!()
    }
}

#[typetag::serde]
impl ItemSave for SimplePropItem {
    fn load(&self) -> Box<dyn Item>  {
        Box::new(self.clone())
    }

}

#[async_trait::async_trait]
impl Drawable for SimplePropItem {


    async fn draw(&mut self, draw_context: &DrawContext ) {
        todo!()
    }
    fn draw_layer(&self) -> u32 {
        todo!()
    }
}


