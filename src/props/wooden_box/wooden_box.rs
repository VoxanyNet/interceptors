use std::path::PathBuf;

use crate::{base_prop::{self, BaseProp}, prop::Prop};
use delegate::delegate;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

pub struct WoodenBox {
    base_prop: BaseProp
}

impl Prop for WoodenBox {
    delegate! {
        to self.base_prop {
            fn name(&self) -> String;
            fn rigid_body_handle(&self) -> RigidBodyHandle;
            fn collider_handle(&self) -> ColliderHandle;
            fn sprite_path(&self) -> PathBuf;
        }
    }
}