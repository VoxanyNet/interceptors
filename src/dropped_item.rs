use glamx::Pose2;
use macroquad::{color::WHITE, math::Vec2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{Prefabs, TextureLoader, area::AreaId, computer::{Item, ItemSave}, drawable::{DrawContext, Drawable}, rapier_to_macroquad, space::Space, texture_loader::ClientTextureLoader, uuid_u64};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DroppedItemId {
    id: u64
}

impl DroppedItemId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }

    pub fn owner_tick(&mut self, space: &mut Space) {

    }
}

#[derive(Clone)]
pub struct DroppedItem {
    pub(crate) item: Item,
    pub body: RigidBodyHandle,
    collider: ColliderHandle,
    pub id: DroppedItemId,
    pub size: glamx::Vec2,
    previous_velocity: RigidBodyVelocity<f32>,
    pub despawn: bool

}



impl DroppedItem {

    pub fn despawn_callback(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn set_velocity(&mut self, space: &mut Space, vel: RigidBodyVelocity<f32>) {
        space.rigid_body_set.get_mut(self.body).unwrap().set_vels(vel, true);
    }

    pub fn from_save(
        save: DroppedItemSave, 
        space: &mut Space, 
        prefabs: &Prefabs,
        textures: TextureLoader
    ) -> Self {

        let item = Item::from_save(save.item, space, textures);

        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(save.pos)
                .linvel(save.velocity.linvel)
                .angvel(save.velocity.angvel)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(
                save.size.x, 
                save.size.y,
            )
                .mass(200.), 
            rigid_body, 
            &mut space.rigid_body_set
        );


        Self {
            item,
            body: rigid_body,
            collider,
            id: save.id,
            previous_velocity: RigidBodyVelocity::zero(),
            size: save.size,
            despawn: false
        }
    }

    pub fn save(&self, space: &Space) -> DroppedItemSave {

        let body = space.rigid_body_set.get(self.body).unwrap();

        let pos = body.position();
        let velocity = body.vels();

        let collider = space.collider_set.get(self.collider).unwrap();

        let collider_hx = collider.shape().as_cuboid().unwrap().half_extents;

        let item_save = self.item.save(space);

        DroppedItemSave {
            pos: *pos,
            item: item_save,
            velocity: *velocity,
            id: self.id.clone(),
            size: collider_hx

        }
    }
    pub fn new(item: Item, pos: Pose2, vel: RigidBodyVelocity<f32>, space: &mut Space, textures: &ClientTextureLoader, prefabs: &Prefabs, size: f32) -> Self {

        let preview_size = item.get_preview_resolution(textures, size, prefabs);

        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos)
                .linvel(vel.linvel)
                .angvel(vel.angvel)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(preview_size.x, preview_size.y)
                .mass(2000.), 
            rigid_body, 
            &mut space.rigid_body_set
        );

        Self {
            item,
            body: rigid_body,
            collider,
            id: DroppedItemId::new(),
            previous_velocity: RigidBodyVelocity::zero(),
            size: glamx::Vec2::new(preview_size.x, preview_size.y),
            despawn: false
        }
    }
}

#[async_trait::async_trait]
impl Drawable for DroppedItem {
    async fn draw(&mut self, draw_context: &DrawContext) {
        if self.despawn == true {
            return;
        }

        let body = draw_context.space.rigid_body_set.get(self.body).unwrap();

        let pos = body.position();

        let half_extents = draw_context.space.collider_set.get(self.collider).unwrap().shape().as_cuboid().unwrap().half_extents;

        // preview uses macroquad coords
        let macroquad_pos = rapier_to_macroquad(pos.translation);

        let macroquad_rotation = pos.rotation.angle() * -1.;

        // this is stupid

        let size = match self.size.x >= self.size.y {
            true => self.size.x * 2.,
            false => self.size.y * 2.,
        };

        self.item.draw_preview(
            draw_context.textures, 
            size, 
            Vec2 {
                x: macroquad_pos.x - half_extents.x,
                y: macroquad_pos.y - half_extents.y,
            }, 
            draw_context.prefabs, 
            Some(WHITE),
            macroquad_rotation
        );
    }

    fn draw_layer(&self) -> u32 {
        1
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DroppedItemSave {
    pos: Pose2,
    item: ItemSave,
    velocity: RigidBodyVelocity<f32>,
    id: DroppedItemId,
    size: glamx::Vec2
       
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewDroppedItemUpdate {
    pub dropped_item: DroppedItemSave,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoveDroppedItemUpdate {
    pub dropped_item_id: DroppedItemId,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DroppedItemVelocityUpdate {
    pub area_id: AreaId,
    pub id: DroppedItemId,
    pub velocity: RigidBodyVelocity<f32>
}
