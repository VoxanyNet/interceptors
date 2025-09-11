use std::{collections::HashSet, path::{Path, PathBuf}, time::Instant};

use macroquad::{audio::{load_sound, load_sound_from_bytes, play_sound, play_sound_once, PlaySoundParams}, color::Color, input::{is_key_released, KeyCode}, math::Vec2, rand::RandomRange};
use nalgebra::{point, vector, Isometry2, Vector2};
use rapier2d::{math::{Translation, Vector}, parry::query::Ray, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};
use serde::{Deserialize, Serialize};

use crate::{area::AreaId, bullet_trail::{self, BulletTrail, SpawnBulletTrail}, collider_from_texture_size, draw_preview, draw_texture_onto_physics_body, dropped_item::{DroppedItem, NewDroppedItemUpdate}, enemy::{Enemy, EnemyId}, get_preview_resolution, inventory::Inventory, machine_gun::{LMGItem, LMGItemSave, LMGSave, LMG}, player::{ActiveWeaponUpdate, Facing, Player, PlayerId}, prop::{DissolvedPixel, Prop, PropVelocityUpdate}, shotgun::{Shotgun, ShotgunItem, ShotgunItemSave, ShotgunSave}, space::Space, texture_loader::TextureLoader, ClientId, ClientTickContext, Prefabs};

pub struct WeaponFireContext<'a> {
    pub space: &'a mut Space,
    pub players: &'a mut Vec<Player>,
    pub props: &'a mut Vec<Prop>,
    pub bullet_trails: &'a mut Vec<BulletTrail>,
    pub facing: Facing,
    pub area_id: AreaId,
    pub dissolved_pixels: &'a mut Vec<DissolvedPixel>,
    pub enemies: &'a mut Vec<Enemy>,
    pub weapon_owner: WeaponOwner
}

#[derive(Clone)]
pub struct BulletImpactData {
    pub shooter_pos: Isometry2<f32>,
    pub impacted_collider: ColliderHandle,
    pub bullet_vector: Vector2<f32>,
    pub damage: f32,
    pub knockback: f32
} 


#[derive(PartialEq, Clone, Debug)]
pub enum WeaponType {
    Shotgun(Shotgun),
    LMG(LMG)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WeaponTypeSave {
    Shotgun(ShotgunSave),
    LMG(LMGSave)
}

#[derive(PartialEq, Clone, Debug)]
pub enum WeaponTypeItem {
    Shotgun(ShotgunItem),
    LMG(LMGItem)
}

impl WeaponTypeItem {

    pub fn stackable(&self) -> bool {
        false
    }

    pub fn handle_existing_weapon(
        &mut self, 
        existing_weapon: &mut Option<WeaponType>,
        ctx: &mut ClientTickContext,
        dropped_items: &mut Vec<DroppedItem>,
        inventory: &mut Inventory,
        area_id: AreaId,
        space: &mut Space,
        player_id: crate::player::PlayerId,
        player_rigid_body_handle: RigidBodyHandle,
        quantity: &mut u32
    ) {
        // drop existing weapon or put in inventory 
        if let Some(existing_weapon) = existing_weapon {
            let item = inventory.try_insert_into_inventory(
                crate::computer::Item::Weapon(
                    existing_weapon.to_item(space)
                ), 
                ctx, 
                area_id, 
                space, 
                player_id
            );

            if let Some(item) = item {

                let player_body = space.rigid_body_set.get(player_rigid_body_handle).unwrap();

                let player_pos = player_body.position();
                let player_velocity = player_body.vels();

                let dropped_item = DroppedItem::new(item, *player_pos, *player_velocity, space, ctx.textures, ctx.prefabs, 20.);

                dropped_items.push(dropped_item.clone());
            
                ctx.network_io.send_network_packet(crate::updates::NetworkPacket::NewDroppedItemUpdate(
                    NewDroppedItemUpdate {
                        dropped_item: dropped_item.save(space),
                        area_id,
                    }
                ));
            } else {
                *quantity += 1;
            }
        }
    }
    pub fn use_item(
        &mut self, 
        existing_weapon: &mut Option<WeaponType>,
        ctx: &mut ClientTickContext,
        dropped_items: &mut Vec<DroppedItem>,
        inventory: &mut Inventory,
        area_id: AreaId,
        space: &mut Space,
        player_id: crate::player::PlayerId,
        player_rigid_body_handle: RigidBodyHandle,
        quantity: &mut u32
    ) {
        
        self.handle_existing_weapon(existing_weapon, ctx, dropped_items, inventory, area_id, space, player_id, player_rigid_body_handle, quantity);
        
        *existing_weapon = Some(self.to_weapon(space, Isometry2::default(), *ctx.client_id, Some(player_rigid_body_handle)));
        
        ctx.network_io.send_network_packet(
            crate::updates::NetworkPacket::ActiveWeaponUpdate(
                ActiveWeaponUpdate {
                    area_id,
                    player_id,
                    weapon: match existing_weapon {
                        Some(existing_weapon) => Some(existing_weapon.save(space)),
                        None => None,
                    },
                }
            )
        );

        *quantity -= 1;
    }

    pub fn name(&self) -> String {
        match self {
            WeaponTypeItem::Shotgun(shotgun) => shotgun.preview_name(),
            WeaponTypeItem::LMG(lmg) => "LMG".to_string()
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        match self {
            WeaponTypeItem::Shotgun(shotgun) => shotgun.draw_preview(textures, size, draw_pos, color, rotation),
            WeaponTypeItem::LMG(lmg) => lmg.draw_preview(textures, size, draw_pos, color, rotation),
        }
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        match self {
            WeaponTypeItem::Shotgun(shotgun) => shotgun.get_preview_resolution(size, textures),
            WeaponTypeItem::LMG(lmg) => lmg.get_preview_resolution(size, textures)
        }
    }

    pub fn save(&self, space: &Space) -> WeaponTypeItemSave {
        match self {
            WeaponTypeItem::Shotgun(shotgun_item) => WeaponTypeItemSave::Shotgun(shotgun_item.save()),
            WeaponTypeItem::LMG(lmg_item) => WeaponTypeItemSave::LMG(lmg_item.save())
        }
    }

    pub fn from_save(save: WeaponTypeItemSave) -> Self {
        match save {
            WeaponTypeItemSave::Shotgun(shotgun_item_save) => WeaponTypeItem::Shotgun(
                ShotgunItem::from_save(shotgun_item_save)
            ),
            WeaponTypeItemSave::LMG(lmg_item_save) => WeaponTypeItem::LMG(
                LMGItem::from_save(lmg_item_save)
            )
        }
    }

    pub fn to_weapon(&self, space: &mut Space, pos: Isometry2<f32>, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>) -> WeaponType {
        match self {
            WeaponTypeItem::Shotgun(shotgun_item) => WeaponType::Shotgun(shotgun_item.to_shotgun(space, pos, owner, player_rigid_body_handle)),
            WeaponTypeItem::LMG(lmg_item) => WeaponType::LMG(lmg_item.to_lmg(space, pos, owner, player_rigid_body_handle))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WeaponTypeItemSave {
    Shotgun(ShotgunItemSave),
    LMG(LMGItemSave)
}

impl WeaponType {

    pub fn name(&self) -> String {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.preview_name(),
            WeaponType::LMG(lmg) => "LMG".to_string()
        }
    }


    pub fn collider_handle(&self) -> ColliderHandle {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.weapon.collider,
            WeaponType::LMG(lmg)  => lmg.weapon.collider
        }
    }

    pub fn despawn(&mut self, space: &mut Space) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.despawn(space),
            WeaponType::LMG(lmg) => lmg.weapon.despawn(space),
        }
    }
    pub fn to_item(&self, space: &Space) -> WeaponTypeItem {
        match self {
            WeaponType::Shotgun(shotgun) => {
                WeaponTypeItem::Shotgun(shotgun.to_item(space))
            },
            WeaponType::LMG(lmg) => {
                WeaponTypeItem::LMG(lmg.to_item(space))
            }
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.draw_preview(textures, size, draw_pos, color, rotation),
            WeaponType::LMG(lmg) => {}
        }
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.get_preview_resolution(size, textures),
            WeaponType::LMG(lmg) => Vec2::ZERO 
        }
    }
    pub fn save(&self, space: &Space) -> WeaponTypeSave {
        return match self {
            WeaponType::Shotgun(shotgun) => WeaponTypeSave::Shotgun(shotgun.save(space)),
            WeaponType::LMG(lmg) => WeaponTypeSave::LMG(lmg.save(space))
        }
    }

    pub fn from_save(space: &mut Space, save: WeaponTypeSave, player_rigid_body_handle:Option<RigidBodyHandle> ) -> Self {
        return match save {
            WeaponTypeSave::Shotgun(shotgun_save) => WeaponType::Shotgun(Shotgun::from_save(shotgun_save, space, player_rigid_body_handle)),
            WeaponTypeSave::LMG(lmg_save) => WeaponType::LMG(LMG::from_save(lmg_save, space, player_rigid_body_handle))
        }
    }

    pub fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.player_joint_handle(),
            WeaponType::LMG(lmg) => lmg.weapon.player_joint_handle
        }
    }

    pub fn rigid_body_handle(&self) -> RigidBodyHandle {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.rigid_body_handle(),
            WeaponType::LMG(lmg) => lmg.weapon.rigid_body
        }
    }
    pub fn fire(&mut self, ctx: &mut ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.fire(ctx, weapon_fire_context),
            WeaponType::LMG(lmg) => lmg.fire(ctx, weapon_fire_context)
        }
    }

    pub fn reload(&mut self) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.reload(),
            WeaponType::LMG(lmg) => lmg.reload(),
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader, facing: Facing) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.draw(space, textures, facing).await,
            WeaponType::LMG(lmg) => lmg.draw(space, textures, facing).await
        }
    }

}




