use derive_more::From;
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};

use crate::{TextureLoader, area::AreaContext, base_prop_save::BasePropSave, material_loader::MaterialLoader, prop::Prop, props::wooden_box::wooden_box_save::WoodenBoxSave, space::Space};

#[typetag::serde(tag = "type")]
pub trait PropSave: DynClone {
    fn load(&self, 
        space: &mut Space, 
        textures: TextureLoader
    ) -> Box<dyn Prop>;
}

dyn_clone::clone_trait_object!(PropSave);