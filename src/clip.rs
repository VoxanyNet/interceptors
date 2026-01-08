use macroquad::{color::WHITE, math::{Rect, Vec2}};
use nalgebra::vector;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{draw_hitbox, drawable::{DrawContext, Drawable}, editor_context_menu::{DataEditorContext, EditorContextMenu, EditorContextMenuData}, rapier_to_macroquad, space::Space};


pub struct Clip {
    pub collider_handle: ColliderHandle,
    pub rigid_body_handle: RigidBodyHandle,
    pub despawn: bool,
    pub context_menu_data: Option<EditorContextMenuData>,
    pub layer: u32
}

impl Clip {

    pub fn despawn_callback(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(
            self.rigid_body_handle, 
            &mut space.island_manager, 
            &mut space.collider_set, 
            &mut space.impulse_joint_set, 
            &mut space.multibody_joint_set, 
            true
        );
    }
    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }
    pub fn from_save(save: ClipSave, space: &mut Space) -> Self {

        let rigid_body_handle = space.rigid_body_set.insert(
            RigidBodyBuilder::fixed()
                .position(vector![save.pos.x, save.pos.y].into())
        );

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(
                save.size.x / 2., 
                save.size.y / 2.
            ),
            rigid_body_handle,
            &mut space.rigid_body_set
        );


        Self {
            rigid_body_handle,
            collider_handle,
            despawn: false,
            context_menu_data: None,
            layer: save.layer
        }
    }

    pub fn save(&self, space: &Space) -> ClipSave {

        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let shape = collider.shape().as_cuboid().unwrap();

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let position = body.position();
        ClipSave {
            size: Vec2::new(shape.half_extents.x * 2., shape.half_extents.y * 2.),
            pos: Vec2::new(position.translation.x, position.translation.y),
            layer: self.layer
        }
    }
}

impl EditorContextMenu for Clip {

    fn collider(&mut self) -> Option<ColliderHandle> {
        Some(self.collider_handle)
    }

    fn rigid_body(&mut self) -> Option<RigidBodyHandle> {
        Some(self.rigid_body_handle)
    }
    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {

        let space = space.unwrap();

        let pos = space.rigid_body_set.get(self.rigid_body_handle).unwrap().translation();
        let size = space.collider_set.get(self.collider_handle).unwrap().shape().as_cuboid().unwrap().half_extents;

        let mpos = rapier_to_macroquad(*pos);

        Rect::new(mpos.x - size.x, mpos.y - size.y, size.x * 2., size.y * 2.)
    }

    fn context_menu_data_mut(&mut self) -> &mut Option<crate::editor_context_menu::EditorContextMenuData> {
        &mut self.context_menu_data
    }

    fn context_menu_data(&self) -> &Option<crate::editor_context_menu::EditorContextMenuData> {
        &self.context_menu_data
    }

    fn despawn(&mut self) -> Option<&mut bool> {
        Some(&mut self.despawn)
    }

    fn data_editor_export(&self, ctx: &DataEditorContext) -> Option<String> {
        Some(serde_json::to_string_pretty(&self.save(&ctx.space)).unwrap())
    }

    fn data_editor_import(&mut self, json: String, ctx: &mut DataEditorContext) {
        let clip_save: ClipSave = serde_json::from_str(&json).unwrap();

        *self = Self::from_save(clip_save, &mut ctx.space)
    }   
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClipSave {
    pub size: Vec2,
    pub pos: Vec2,
    pub layer: u32
}
#[async_trait::async_trait]
impl Drawable for Clip {
    async fn draw(&mut self, draw_context: &DrawContext) {
        let mut color = WHITE;

        color.a = 0.2;

        draw_hitbox(&draw_context.space, self.rigid_body_handle, self.collider_handle, color);
    }

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}