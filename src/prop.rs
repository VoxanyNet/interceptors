use std::{default, fs::read_to_string, path::PathBuf, time::Instant};

use macroquad::{audio::play_sound_once, color::Color, input::is_mouse_button_released, math::Vec2, shapes::{draw_rectangle_ex, DrawRectangleParams}};
use nalgebra::{Isometry2, Vector, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, ColliderPair, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{area::AreaId, contains_point, draw_texture_onto_physics_body, rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, weapon::BulletImpactData, ClientId, ClientTickContext, ServerIO};

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub enum PropMaterial {
    Wood,
    #[default]
    None
}

pub struct DissolvedPixel {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
    color: Color,
    scale: f32
}


impl DissolvedPixel {

    pub fn new(
        pos: Isometry2<f32>, 
        space: &mut Space,
        color: Color,
        scale: f32,
        mass: Option<f32>,
        velocity: Option<RigidBodyVelocity>
    ) -> Self {

        let velocity = match velocity {
            Some(velocity) => velocity,
            None => RigidBodyVelocity::zero(),
        };

        let mass = match mass {
            Some(mass) => mass,
            None => 1.,
        };

        

        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos)
                .additional_mass(mass)
                .angvel(velocity.angvel)
                .linvel(velocity.linvel)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(1., 1.),
            rigid_body,
            &mut space.rigid_body_set
        );

        Self {
            body: rigid_body,
            collider,
            color,
            scale,
        }
    }
    pub fn draw(&self, space: &Space) {

        let body = space.rigid_body_set.get(self.body).unwrap();

        let macroquad_pos = rapier_to_macroquad(*body.translation());

        let shape = space.collider_set.get(self.collider).unwrap().shape().as_cuboid().unwrap();


        draw_rectangle_ex(
            macroquad_pos.x, 
            macroquad_pos.y, 
            (shape.half_extents.x * 2.) * self.scale, 
            (shape.half_extents.y * 2.) * self.scale, 
            DrawRectangleParams { 
                offset: macroquad::math::Vec2::new(0.5, 0.5), 
                rotation: body.rotation().angle() * -1., 
                color: self.color 
            }
        );
        
    }
}

pub struct Prop {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    sprite_path: PathBuf,
    previous_velocity: RigidBodyVelocity,
    material: PropMaterial,
    pub id: PropId,
    pub owner: Option<ClientId>,
    last_sound_play: web_time::Instant,
    pub despawn: bool 
}

impl Prop {

    pub fn handle_bullet_impact(
        &mut self, 
        ctx: &mut ClientTickContext, 
        impact: &BulletImpactData, 
        space: &mut Space, 
        area_id: AreaId,
        dissolved_pixels: &mut Vec<DissolvedPixel>
    ) {

        if self.despawn {
            return;
        }

        let rigid_body = space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap();

        rigid_body.apply_impulse(
            Vector2::new(impact.bullet_vector.x * 5000., impact.bullet_vector.y * 5000.), 
            true
        );

        //manually create a prop velocity update if we dont own it (because if we do it just happens automatically)

        match self.owner {
            Some(owner) => {
                if *ctx.client_id != owner {
                    ctx.network_io.send_network_packet(
                        crate::updates::NetworkPacket::PropVelocityUpdate(
                            PropVelocityUpdate {
                                velocity: *rigid_body.vels(),
                                id: self.id,
                                area_id,
                            }
                        )
                    );
                }
            },
            None => {},
        }


        self.dissolve(ctx.textures, space, dissolved_pixels);

        self.despawn(space);
    }
    
    pub fn despawn(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);

        self.despawn = true;


    }
    pub fn from_prefab(prefab_path: String, space: &mut Space) -> Self {

        let prop_save: PropSave = serde_json::from_str(&read_to_string(prefab_path).unwrap()).unwrap();

        let prop = Prop::from_save(prop_save, space);

        prop
    }
    pub fn server_tick(&mut self, space: &mut Space, area_id: AreaId, server_io: &mut ServerIO) {

    }

    pub fn dissolve(&mut self, textures: &TextureLoader, space: &mut Space, dissolved_pixels: &mut Vec<DissolvedPixel>) {

        let collider = space.collider_set.get(self.collider_handle).unwrap().clone();
        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap().clone();

        let body_translation = body.translation();

        let texture = textures.get(&self.sprite_path);

        let half_extents = collider.shape().as_cuboid().unwrap().half_extents;

        let x_scale = (half_extents.x * 2.) / texture.width() ;

        let y_scale = (half_extents.y * 2.) / texture.height();

        let texture_data = texture.get_texture_data();

        let total_pixel_count = texture.width() * texture.height();

        for x in 0..texture.width() as u32 {
            for y in 0..texture.height() as u32 {
                let color = texture_data.get_pixel(x, y);

                let translation = Vector2::new(
                ((body_translation.x + (x as f32 * x_scale)) - half_extents.x) + 0.5, 
                ((body_translation.y - (y as f32 * y_scale)) + half_extents.y) - 0.5    
                );

                let position = Isometry2::new(
                    translation, 
                    body.rotation().angle()
                );

                dissolved_pixels.push(
                    DissolvedPixel::new(
                        position, // do we need to shift this over by 0.5?  
                        space, 
                        color, 
                        x_scale, 
                        Some(collider.mass() / total_pixel_count), // this needs to be divided by the number of pixels 
                        Some(*body.vels())
                    )
                );

            }
        }

        self.despawn(space);

        println!("dissolved");
    }


    pub fn play_impact_sound(&mut self, space: &Space, ctx: &mut ClientTickContext) {
        for contact_pair in space.narrow_phase.contact_graph().interactions().into_iter() {
            if contact_pair.collider1 == self.collider_handle || contact_pair.collider2 == self.collider_handle {
                // dbg!(&contact_pair.manifolds);
                // dbg!(&contact_pair.total_impulse());

                if contact_pair.total_impulse_magnitude() > 2500. && self.last_sound_play.elapsed().as_secs() > 1 {

                    
                    
                    play_sound_once(ctx.sounds.get(PathBuf::from("assets\\sounds\\crate\\tap.wav")));

                    self.last_sound_play = web_time::Instant::now();
                }
            }
        }
    }

    pub fn owner_tick(&mut self, ctx: &mut ClientTickContext, space: &mut Space, area_id: AreaId, dissolved_pixels: &mut Vec<DissolvedPixel>) {

        self.play_impact_sound(space, ctx);

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

    pub fn client_tick(&mut self, space: &mut Space, area_id: AreaId, ctx: &mut ClientTickContext, dissolved_pixels: &mut Vec<DissolvedPixel>) {

        if self.despawn {
            return;
        }

        if let Some(owner) = self.owner {
            if owner == *ctx.client_id {
                self.owner_tick(ctx, space, area_id, dissolved_pixels);
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
            material: save.material,
            owner: save.owner,
            last_sound_play: web_time::Instant::now(),
            despawn: false
            
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
            owner: self.owner,
            material: self.material
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {

        if self.despawn {
            return;
        }
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
    pub sprite_path: PathBuf,
    pub id: Option<PropId>,
    #[serde(default)]
    pub owner: Option<ClientId>,
    #[serde(default)]
    pub material: PropMaterial
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq)]
pub struct PropId {
    id: u64
}

impl PropId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
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



