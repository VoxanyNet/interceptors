use macroquad::{color::Color, math::Vec2};
use nalgebra::Isometry2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{area::AreaId, dropped_item::{DroppedItem, NewDroppedItemUpdate}, inventory::Inventory, player::ActiveWeaponUpdate, space::Space, texture_loader::TextureLoader, weapons::{lmg::item::LMGItem, shotgun::item::ShotgunItem, weapon_type::WeaponType, weapon_type_item_save::WeaponTypeItemSave}, ClientId, ClientTickContext};

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






