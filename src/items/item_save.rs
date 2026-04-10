use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::items::Item;

#[typetag::serde(tag = "type")]
pub trait ItemSave: DynClone {
    fn load(&self) -> Box<dyn Item>;
}

dyn_clone::clone_trait_object!(ItemSave);