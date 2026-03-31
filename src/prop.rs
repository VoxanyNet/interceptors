use std::path::PathBuf;

use downcast_rs::{Downcast, impl_downcast};
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};


impl_downcast!(Prop);
pub trait Prop: Downcast {
    fn name(&self) -> String;
    fn rigid_body_handle(&self) -> RigidBodyHandle;
    fn collider_handle(&self) -> ColliderHandle;
    fn sprite_path(&self) -> PathBuf;
    
}