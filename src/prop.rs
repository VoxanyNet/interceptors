use std::{f32::consts::E, fs::read_to_string};

use macroquad::{math::Vec2, miniquad::window::quit};
use nalgebra::{vector, Isometry2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{area::AreaId, draw_texture_onto_physics_body, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, ClientId, ClientTickContext, ServerIO};

pub struct Prop {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    sprite_path: String,
    previous_velocity: RigidBodyVelocity,
    pub id: PropId,
    pub owner: Option<ClientId>
}

impl Prop {
    
    pub fn from_prefab(prefab_path: String, space: &mut Space) -> Self {

        let prop_save: PropSave = serde_json::from_str(&read_to_string(prefab_path).unwrap()).unwrap();

        let prop = Prop::from_save(prop_save, space);

        prop
    }
    pub fn server_tick(&mut self, space: &mut Space, area_id: AreaId, server_io: &mut ServerIO) {

    }

    pub fn owner_tick(&mut self, ctx: &mut ClientTickContext, space: &mut Space, area_id: AreaId) {

        let current_velocity = *space.rigid_body_set.get(self.rigid_body_handle).unwrap().vels();

        if current_velocity != self.previous_velocity {
            //println!("sending pos update");
            ctx.network_io.send_network_packet (
                NetworkPacket::PropVelocityUpdate(
                    PropVelocityUpdate {
                        velocity: current_velocity,
                        id: self.id,
                        area_id: area_id
                    }
                )
            );
        }
    }   

    pub fn client_tick(&mut self, space: &mut Space, area_id: AreaId, ctx: &mut ClientTickContext) {

        if let Some(owner) = self.owner {
            if owner == *ctx.client_id {
                self.owner_tick(ctx, space, area_id);
            }
        }

        let current_velocity = *space.rigid_body_set.get(self.rigid_body_handle).unwrap().vels();

        self.previous_velocity = current_velocity;
    }
    pub fn set_pos(&mut self, position: Isometry2<f32>, space: &mut Space) {
        space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap().set_position(position, true);
    }

    pub fn set_velocity(&mut self, velocity: RigidBodyVelocity, space: &mut Space) {
        space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap().set_vels(velocity, true);
    }

    pub fn from_save(save: PropSave, space: &mut Space) -> Self {

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(save.pos)
                .ccd_enabled(true)
                .soft_ccd_prediction(20.)
        );


        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(save.size.x / 2., save.size.y / 2.)
                .mass(save.mass),
            body,
            &mut space.rigid_body_set
        );

        let id = match save.id {
            Some(id) => id,
            None => PropId::new(),
        };

        Self {
            rigid_body_handle: body,
            collider_handle: collider,
            sprite_path: save.sprite_path,
            previous_velocity: RigidBodyVelocity::zero(),
            id,
            owner: save.owner
            
        }
    }

    pub fn save(&self, space: &Space) -> PropSave {

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let pos = body.position().clone();
        
        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let mass = collider.mass();
        let size = collider.shape().as_cuboid().unwrap().half_extents;

        PropSave {
            size: Vec2::new(size.x * 2., size.y * 2.),
            pos,
            mass,
            sprite_path: self.sprite_path.clone(),
            id: Some(self.id.clone()),
            owner: self.owner
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {


        draw_texture_onto_physics_body(
            self.rigid_body_handle, 
            self.collider_handle, 
            space, 
            &self.sprite_path, 
            textures, 
            false, 
            false, 
            0.
        ).await;

    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropSave {
    pub size: Vec2,
    pub pos: Isometry2<f32>,
    pub mass: f32,
    pub sprite_path: String,
    pub id: Option<PropId>,
    #[serde(default)]
    pub owner: Option<ClientId>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq)]
pub struct PropId {
    id: u64
}

impl PropId {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u64_pair().0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropVelocityUpdate {
    pub velocity: RigidBodyVelocity,
    pub id: PropId,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropUpdateOwner {
    pub owner: Option<ClientId>,
    pub id: PropId,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewProp {
    pub prop: PropSave,
    pub area_id: AreaId
}



