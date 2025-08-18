use serde::{Deserialize, Serialize};
use crate::{area::{AreaId, AreaSave}, player::{NewPlayer, PlayerCursorUpdate, PlayerFacingUpdate, PlayerVelocityUpdate}, prop::{NewProp, PropUpdateOwner, PropVelocityUpdate}, uuid_u64};

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
    PlayerFacingUpdate(PlayerFacingUpdate)
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