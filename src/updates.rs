use serde::{Deserialize, Serialize};
use crate::{area::{AreaId, AreaSave}, bullet_trail::SpawnBulletTrail, dropped_item::{DroppedItemVelocityUpdate, NewDroppedItemUpdate}, player::{NewPlayer, PlayerCursorUpdate, PlayerFacingUpdate, PlayerPositionUpdate, PlayerVelocityUpdate}, prop::{DissolveProp, NewProp, PropPositionUpdate, PropUpdateOwner, PropVelocityUpdate, RemovePropUpdate}, uuid_u64};

#[derive(Serialize, Deserialize, Clone)]
pub enum NetworkPacket {
    Ping(Ping),
    LoadArea(LoadArea),
    PropVelocityUpdate(PropVelocityUpdate),
    PropUpdateOwner(PropUpdateOwner),
    NewProp(NewProp),
    PlayerVelocityUpdate(PlayerVelocityUpdate),
    NewPlayer(NewPlayer),
    PlayerCursorUpdate(PlayerCursorUpdate),
    PlayerFacingUpdate(PlayerFacingUpdate),
    SpawnBulletTrail(SpawnBulletTrail),
    PlayerPositionUpdate(PlayerPositionUpdate),
    PropPositionUpdate(PropPositionUpdate),
    RemovePropUpdate(RemovePropUpdate),
    DissolveProp(DissolveProp),
    DroppedItemVelocityUpdate(DroppedItemVelocityUpdate),
    NewDroppedItemUpdate(NewDroppedItemUpdate)
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Ping {
    pub id: u64
}

impl Ping {
    pub fn new() -> Self {
        Self {
            id: uuid_u64()
        }
    }

    pub fn new_with_id(id: u64) -> Self {
        Self {
            id,
        }
    }
}

#[derive(Serialize, Deserialize,  Clone)]
pub struct LoadArea {
    pub area: AreaSave,
    pub id: AreaId
}


// Option for prop id in save only, must be actual id when loaded into game, None when needs to be set yourself