use macroquad::{color::WHITE, math::Vec2};
use nalgebra::{Isometry2, Vector2, U4};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{area::AreaId, computer::{Item, ItemSave}, draw_texture_onto_physics_body, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, uuid_u64, Prefabs};

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
    pub size: Vector2<f32>,
    previous_velocity: RigidBodyVelocity,
    despawn: bool

}



impl DroppedItem {

    pub fn despawn(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);

        self.despawn = true;
    }

    pub fn set_velocity(&mut self, space: &mut Space, vel: RigidBodyVelocity) {
        space.rigid_body_set.get_mut(self.body).unwrap().set_vels(vel, true);
    }

    pub fn from_save(save: DroppedItemSave, space: &mut Space, prefabs: &Prefabs) -> Self {

        let item = Item::from_save(save.item, space);

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
    pub fn new(item: Item, pos: Isometry2<f32>, vel: RigidBodyVelocity, space: &mut Space, textures: &TextureLoader, prefabs: &Prefabs, size: f32) -> Self {

        let preview_size = item.get_preview_resolution(textures, size, prefabs);

        dbg!(preview_size);

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
            size: Vector2::new(preview_size.x, preview_size.y),
            despawn: false
        }
    }

    pub fn draw(&self, space: &Space, textures: &TextureLoader, prefabs: &Prefabs) {

        if self.despawn == true {
            return;
        }

        let body = space.rigid_body_set.get(self.body).unwrap();

        let pos = body.position();

        let half_extents = space.collider_set.get(self.collider).unwrap().shape().as_cuboid().unwrap().half_extents;

        // preview uses macroquad coords
        let macroquad_pos = rapier_to_macroquad(pos.translation.vector);

        let macroquad_rotation = pos.rotation.angle() * -1.;

        // this is stupid

        let size = match self.size.x >= self.size.y {
            true => self.size.x * 2.,
            false => self.size.y * 2.,
        };

        self.item.draw_preview(
            textures, 
            size, 
            Vec2 {
                x: macroquad_pos.x - half_extents.x,
                y: macroquad_pos.y - half_extents.y,
            }, 
            prefabs, 
            Some(WHITE),
            macroquad_rotation
        );
            
        
        
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DroppedItemSave {
    pos: Isometry2<f32>,
    item: ItemSave,
    velocity: RigidBodyVelocity,
    id: DroppedItemId,
    size: Vector2<f32>
       
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewDroppedItemUpdate {
    pub dropped_item: DroppedItemSave,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DroppedItemVelocityUpdate {
    pub area_id: AreaId,
    pub id: DroppedItemId,
    pub velocity: RigidBodyVelocity
}