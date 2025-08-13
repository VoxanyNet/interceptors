use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{area::{AreaId, AreaSave}, player::{NewPlayer, PlayerCursorUpdate, PlayerPositionUpdate}, prop::{NewProp, PropPosUpdate, PropUpdateOwner}};

#[derive(Serialize, Deserialize, Clone)]
pub enum NetworkPacket {
    Ping(Ping),
    LoadArea(LoadArea),
    PropPosUpdate(PropPosUpdate),
    PropUpdateOwner(PropUpdateOwner),
    NewProp(NewProp),
    PlayerPositionUpdate(PlayerPositionUpdate),
    NewPlayer(NewPlayer),
    PlayerCursorUpdate(PlayerCursorUpdate)
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Ping {
    pub id: u64
}

impl Ping {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u64_pair().0
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