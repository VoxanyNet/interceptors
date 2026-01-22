
use glamx::vec2;
use macroquad::color::WHITE;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle};

use crate::{draw_hitbox, player::Facing, space::Space, texture_loader::TextureLoader, weapons::{lmg::{ weapon_save::LMGSave}, sledge::weapon_save::SledgeSave}, ClientId};

#[derive(PartialEq, Clone, Debug)]
pub struct Sledge {
    rigid_body: RigidBodyHandle,
    collider: ColliderHandle
}


impl Sledge {

    pub fn new(space: &mut Space, pos: glamx::Vec2, owner: ClientId, player_rigid_body_handle: Option<RigidBodyHandle>) -> Self {
        
        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(10., 5.), 
            body, 
            &mut space.rigid_body_set
        );
        
        if let Some(player_rigid_body_handle) = player_rigid_body_handle {
            space.impulse_joint_set.insert(
                player_rigid_body_handle, 
                body,
                RevoluteJointBuilder::new()
                    .local_anchor1(vec2(0., 0.))
                    .local_anchor2(vec2(30., 0.))
                    .contacts_enabled(false), 
                true
            );
        }
        
        Self {
            rigid_body: body,
            collider: collider,
        }
    }

    pub fn save(&self, space: &Space) -> SledgeSave {
        SledgeSave {


        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader, facing: Facing) {
        // let body = space.rigid_body_set.get(self.rigid_body).unwrap();

        // let collider = space.collider_set.get(self.collider).unwrap();

        draw_hitbox(space, self.rigid_body, self.collider, WHITE);

    }

    pub fn reload(&mut self) {
        
    }

    pub fn from_save(save: LMGSave, space: &mut Space, player_rigid_body_handle: Option<RigidBodyHandle>, owner: ClientId) -> Self {
        Self::new(space, glamx::Vec2::ZERO, owner, player_rigid_body_handle)
    }
    

    
}