use serde::{Deserialize, Serialize};

use crate::{TextureLoader, base_prop_save::BasePropSave, props::wooden_box::wooden_box::WoodenBox, space::Space};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WoodenBoxSave {
    base_prop_save: BasePropSave
}

impl WoodenBoxSave {
    pub fn load(&self, space: &mut Space, textures: TextureLoader) -> WoodenBox {
        WoodenBox {
            base_prop: self.base_prop_save.load(space, textures),
        }
    }
}