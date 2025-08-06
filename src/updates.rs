use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum NetworkPacket {
    Ping(Ping),
    
}

#[derive(Serialize, Deserialize)]
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