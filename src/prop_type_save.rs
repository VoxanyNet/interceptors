use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::{TextureLoader, base_prop_save::BasePropSave, prop::Prop, props::wooden_box::wooden_box_save::WoodenBoxSave, space::Space};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, From)]
pub enum PropTypeSave {
    BasePropSave(BasePropSave), // this needs to be possible for the trait to work im not sure if this is good
    WoodenBoxSave(WoodenBoxSave)
}

impl PropTypeSave {
    pub fn load(&self, space: &mut Space, textures: TextureLoader) -> Box<dyn Prop> {
        match self {
            
            PropTypeSave::WoodenBoxSave(wooden_box_save) => Box::new(wooden_box_save.load(space, textures)),
            PropTypeSave::BasePropSave(base_prop_save) => Box::new(base_prop_save.load(space, textures)),
        }
    }
}