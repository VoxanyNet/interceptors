use serde::{Deserialize, Serialize};

use crate::{TickContext, drawable::{DrawContext, Drawable}, items::{ConsumedStatus, Item, item_save::ItemSave}, prop::Prop, props::wooden_box::wooden_box::WoodenBox, weapons::ItemOwnerContext};



// prop item does not require a seperate prop item save 
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum PropItem {
    WoodenBox
}


impl Item for PropItem {
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
        todo!()
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
        todo!()
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
impl ItemSave for PropItem {
    fn load(&self) -> Box<dyn Item>  {
        Box::new(self.clone())
    }

}

#[async_trait::async_trait]
impl Drawable for PropItem {


    async fn draw(&mut self, draw_context: &DrawContext ) {
        todo!()
    }
    fn draw_layer(&self) -> u32 {
        todo!()
    }
}


