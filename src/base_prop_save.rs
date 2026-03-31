use std::path::PathBuf;

use glamx::Pose2;
use rapier2d::prelude::RigidBodyType;
use serde::{Deserialize, Serialize};

use crate::{Owner, base_prop::{Material, PropId}};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BasePropSave {

    #[serde(default)]
    pub scale: f32,
    pub pos: Pose2,
    pub mass: f32,
    pub sprite_path: PathBuf,
    pub id: Option<PropId>,
    #[serde(default)]
    pub owner: Option<Owner>,
    #[serde(default)]
    pub material: Material,
    #[serde(default = "default_prop_name")]
    pub name: String,
    #[serde(default)]
    pub layer: u32,
    // only provide voxels if they have been modified from what the image would generate
    #[serde(default)]
    pub voxels: Option<Vec<glamx::IVec2>>,
    #[serde(default="default_body_type")]
    pub rigid_body_type: RigidBodyType,
    #[serde(default)]
    pub removed_voxels: Vec<glamx::IVec2>,
    #[serde(default)]
    pub lifespan: Option<web_time::Duration>,
    #[serde(default = "default_sync_physics")]
    pub sync_physics: bool,
}




fn default_sync_physics() -> bool {
    true
}

fn default_body_type() -> RigidBodyType {
    RigidBodyType::Dynamic
}

fn default_prop_name() -> String {
    "Unnamed Prop".to_string()
}