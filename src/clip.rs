use macroquad::math::{Rect, Vec2};
use nalgebra::vector;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use crate::{editor_context_menu::{EditorContextMenu, EditorContextMenuData}, rapier_to_macroquad, space::Space};


pub struct Clip {
    pub collider_handle: ColliderHandle,
    pub rigid_body_handle: RigidBodyHandle,
    pub despawn: bool,
    pub context_menu_data: Option<EditorContextMenuData>
}

impl Clip {
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
            context_menu_data: None
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
        }
    }
}

impl EditorContextMenu for Clip {
    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {

        let space = space.unwrap();

        let pos = space.rigid_body_set.get(self.rigid_body_handle).unwrap().translation();
        let size = space.collider_set.get(self.collider_handle).unwrap().shape().as_cuboid().unwrap().half_extents;

        let mpos = rapier_to_macroquad(*pos);

        Rect::new(mpos.x, mpos.y, size.x, size.y)
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
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipSave {
    pub size: Vec2,
    pub pos: Vec2,
}