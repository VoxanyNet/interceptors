use crate::{area::AreaId, computer::Item, player::{ItemSlot, ItemSlotQuantityUpdate, ItemSlotUpdate, PlayerId}, space::Space, updates::NetworkPacket, ClientTickContext};

pub struct Inventory {
    pub items: [Option<ItemSlot>; 6]
}

impl Inventory {

    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    /// Will try to insert item into inventory but will return if cant
    pub fn try_insert_into_inventory(
        &mut self, 
        item: Item, 
        ctx: &mut ClientTickContext, 
        area_id: AreaId,
        space: &mut Space,
        player_id: PlayerId
    ) -> Option<Item> {
    
        for (item_slot_index, item_slot) in &mut self.items.iter_mut().enumerate() {
            match item_slot {
                Some(item_slot) => {

                    if !item_slot.item.stackable() {
                        
                        continue;
                    }
                    // matching item
                    if item_slot.item == item {
                        item_slot.quantity += 1;

                        ctx.network_io.send_network_packet(
                            NetworkPacket::ItemSlotQuantityUpdate(
                                ItemSlotQuantityUpdate {
                                    area_id: area_id,
                                    player_id,
                                    inventory_index: item_slot_index,
                                    quantity: item_slot.quantity,
                                }
                            )
                        );

                        return None;
                    }

                },
                None => {
                    *item_slot = Some(
                        ItemSlot {
                            quantity: 1,
                            item,
                        }
                    );

                    let item_slot_save = match item_slot {
                        Some(item_slot) => Some(item_slot.save(space)),
                        None => None, 
                    };

                    ctx.network_io.send_network_packet(
                        NetworkPacket::ItemSlotUpdate(
                            ItemSlotUpdate {
                                area_id,
                                player_id: player_id,
                                inventory_index: item_slot_index,
                                item_slot: item_slot_save,
                            }
                        )
                    );

                    return None;
                },
            }
        }

        Some(item)
    }
}