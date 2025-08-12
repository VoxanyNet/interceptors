use serde::{Deserialize, Serialize};

use crate::ClientTickContext;

pub struct Player {
    
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