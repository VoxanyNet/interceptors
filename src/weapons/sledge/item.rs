use std::{path::PathBuf, time::Duration};

use macroquad::{color::Color, math::Vec2};
use nalgebra::Isometry2;
use rapier2d::prelude::RigidBodyHandle;

use crate::{space::Space, texture_loader::TextureLoader, weapons::{lmg::{item_save::LMGItemSave, weapon::LMG}, sledge::{item_save::SledgeItemSave, weapon::Sledge}, weapon::item::WeaponItem}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct SledgeItem {

}

impl SledgeItem {

    pub fn new() -> Self {
        Self {
            
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        
    }    

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        Vec2::ZERO
    }

    pub fn save(&self) -> SledgeItemSave {
        SledgeItemSave {

        }
    }

    pub fn from_save(save: LMGItemSave) -> Self {
        Self {

        }
    }

    pub fn to_sledge(&self, 
        space:&mut Space, 
        pos: Isometry2<f32>, 
        owner: ClientId, 
        player_rigid_body_handle: Option<RigidBodyHandle>
    ) -> Sledge {
        Sledge::new(space, pos.translation.vector, owner, player_rigid_body_handle)
    }

}

impl Default for SledgeItem {
    fn default() -> Self {
        Self::new()
    }
}




