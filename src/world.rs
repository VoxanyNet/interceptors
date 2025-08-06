use serde::{Deserialize, Serialize};

use crate::{area::{Area, AreaSave}, ClientTickContext};

pub struct World {
    areas: Vec<Area>
}

impl World {
    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for arena in &mut self.areas {
            arena.client_tick(ctx)
        }
    }

    pub fn empty() -> Self {
        Self {
            areas: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldSave {
    pub areas: Vec<AreaSave>
}