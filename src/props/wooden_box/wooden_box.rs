use std::path::PathBuf;

use crate::{Owner, TickContext, area::AreaContext, base_prop::{self, BaseProp, Material, PropId}, drawable::Drawable, prop::Prop, prop_save::PropSave, space::Space, texture_loader::ClientTextureLoader, weapons::bullet_impact_data::BulletImpactData};
use async_trait::async_trait;
use delegate::delegate;
use macroquad::math::Rect;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

pub struct WoodenBox {
    pub base_prop: BaseProp
}
#[async_trait]
impl Drawable for WoodenBox {
    async fn draw(&mut self, draw_context: &crate::drawable::DrawContext) {
        self.base_prop.draw(draw_context).await
    }

    fn draw_layer(&self) -> u32 {
        self.base_prop.layer
    }
}

impl Prop for WoodenBox {
    delegate! {
        to self.base_prop {
            fn set_name(&mut self, name: &str);
            fn set_material(&mut self, new_material: Material);
            fn name(&self) -> String;
            fn rigid_body_handle(&self) -> RigidBodyHandle;
            fn collider_handle(&self) -> ColliderHandle;
            fn sprite_path(&self) -> PathBuf;
            fn tick(&mut self, area_context: &mut AreaContext, ctx: &mut TickContext);
            fn id(&self) -> PropId;
            fn should_despawn(&self) -> bool;
            fn despawn_callback(&mut self, space: &mut Space);
            fn last_ownership_change(&self) -> web_time::Instant;
            fn last_ownership_change_mut(&mut self) -> &mut web_time::Instant;
            fn owner(&self) -> Option<Owner>;
            fn owner_mut(&mut self) -> &mut Option<Owner>;
            fn save(&self, space: &Space) -> Box<dyn PropSave>;
            fn handle_bullet_impact(
                &mut self,
                ctx: &mut TickContext,
                area_context: &mut AreaContext,
                impact: &BulletImpactData,
            );
            fn removed_voxels(&self) -> &Vec<glamx::IVec2>;
            fn removed_voxels_mut(&mut self) -> &mut Vec<glamx::IVec2>;
            fn voxels_modified(&self) -> &bool;
            fn voxels_modified_mut(&mut self) -> &mut bool;
            fn last_received_position_update(&self) -> web_time::Instant;
            fn last_received_position_update_mut(&mut self) -> &mut web_time::Instant;
            fn mark_despawn(&mut self);
            fn draw_editor_context_menu(&self);
            fn update_menu(&mut self, space: &mut Space, camera_rect: &Rect, selected: bool, textures: &ClientTextureLoader);
            fn set_mass(&self, space: &mut Space, new_mass: f32);
        }
    }
}