use nalgebra::Isometry2;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{area::AreaId, ClientId, ClientTickContext};

#[derive(Serialize, Deserialize)]
pub struct PlayerId {
    id: u64
}

impl PlayerId {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u64_pair().0,
        }
    }
}

pub struct Player {
    id: PlayerId,
    health: u32,
    
}

impl Player {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {

    }

    pub fn from_save(save: PlayerSave) -> Self {
        Self {
            
        }
    }

    pub fn server_tick(&mut self) {

    }

    pub fn save(&self) -> PlayerSave {
        PlayerSave {  
            
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSave {

}

#[derive(Serialize, Deserialize)]
pub struct PlayerPositionUpdate {
    pub id: PlayerId,
    pub area_id: AreaId,
    pub pos: Isometry2<f32>
}

pub struct PlayerCursorUpdate {
    pub id
}