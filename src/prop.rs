use std::path::PathBuf;

use downcast_rs::{Downcast, impl_downcast};
use macroquad::math::Rect;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::{Owner, TickContext, area::AreaContext, base_prop::{Material, PropId}, drawable::Drawable, prop_save::PropSave, space::Space, texture_loader::ClientTextureLoader, weapons::bullet_impact_data::BulletImpactData};


impl_downcast!(Prop);
pub trait Prop: Downcast {
    fn layer(&self) -> u32;
    fn name(&self) -> String;
    fn rigid_body_handle(&self) -> RigidBodyHandle;
    fn collider_handle(&self) -> ColliderHandle;
    fn sprite_path(&self) -> PathBuf;
    fn tick(&mut self, area_context: &mut AreaContext, ctx: &mut TickContext);
    fn id(&self) -> PropId;
    fn should_despawn(&self) -> bool;
    fn despawn_callback(&mut self, space: &mut Space); // need to add area context but i dont feel like it right now
    fn handle_bullet_impact(
        &mut self,
        ctx: &mut TickContext,
        area_context: &mut AreaContext,
        impact: &BulletImpactData,
    );
    fn save(&self, space: &Space) -> Box<dyn PropSave>;
    fn last_ownership_change(&self) -> web_time::Instant;
    fn last_ownership_change_mut(&mut self) -> &mut web_time::Instant;
    fn owner(&self) -> Option<Owner>;
    fn owner_mut(&mut self) -> &mut Option<Owner>;
    fn removed_voxels(&self) -> &Vec<glamx::IVec2>;
    fn removed_voxels_mut(&mut self) -> &mut Vec<glamx::IVec2>;
    fn voxels_modified(&self) -> &bool;
    fn voxels_modified_mut(&mut self) -> &mut bool;
    fn set_pos(&mut self, position: glamx::Pose2,  space: &mut Space) {
        space.rigid_body_set.get_mut(self.rigid_body_handle()).unwrap()
            .set_position(
                position,
                true
            );
    }
    fn last_received_position_update(&self) -> web_time::Instant;
    fn last_received_position_update_mut(&mut self) -> &mut web_time::Instant;
    fn mark_despawn(&mut self);
    fn draw_editor_context_menu(&self); // maybe we should actually use the trait 
    fn update_menu(&mut self, space: &mut Space, camera_rect: &Rect, selected: bool, textures: &ClientTextureLoader);
    fn set_mass(&self, space: &mut Space, new_mass: f32);
    fn set_material(&mut self, new_material: Material);
    fn set_name(&mut self, name: &str);
    fn draw(&mut self, ctx: &mut TickContext, space: &mut Space);
}