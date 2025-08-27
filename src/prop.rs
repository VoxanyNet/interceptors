use std::{default, fs::read_to_string, path::{Path, PathBuf}, time::Instant};

use macroquad::{audio::play_sound_once, color::{Color, WHITE}, input::is_mouse_button_released, math::Vec2, shapes::{draw_rectangle_ex, DrawRectangleParams}, texture::{draw_texture, draw_texture_ex, DrawTextureParams}};
use nalgebra::{base, Isometry2, Vector, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, ColliderPair, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{area::{Area, AreaId}, computer::Computer, contains_point, draw_preview, draw_texture_onto_physics_body, get_preview_resolution, prop, rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, weapon::BulletImpactData, ClientId, ClientTickContext, Prefabs, ServerIO};

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
    scale: f32,
    spawned: Instant,
    pub despawn: bool,
}


impl DissolvedPixel {

    pub fn client_tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {

        if self.despawn {
            return;
        }

        let elapsed = self.spawned.elapsed().as_secs_f32();
        
        if elapsed == 0. {
            return;
        }


        self.color.a -= 0.01 * elapsed;

        if self.color.a <= 0. {
            self.despawn(space, ctx)
        }

    }

    pub fn despawn(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {

        if self.despawn {
            return;
        }

        self.despawn = true;

        space.rigid_body_set.remove(self.body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

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
                .additional_mass(0.0001)
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
            spawned: Instant::now(),
            despawn: false
        }
    }
    pub fn draw(&self, space: &Space) {

        if self.despawn {
            return;
        }

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
    pub despawn: bool,
    last_pos_update: web_time::Instant,
    name: String
}

#[derive(PartialEq, Clone, Debug)]
pub struct PropItem {
    pub prefab_path: PathBuf
}

impl PropItem {

    pub fn stackable(&self) -> bool {
        true
    }

    pub fn from_save(save: PropItemSave) -> Self {
        PropItem {
            prefab_path: PathBuf::from(save.prefab_path),
        }
    }

    pub fn save(&self) -> PropItemSave {
        PropItemSave {
            prefab_path: self.prefab_path.to_str().unwrap().to_string(),
        }
    }
    pub fn name(&self, prefabs: &Prefabs) -> String {
        let prop_json = prefabs.get_prefab_data(&self.prefab_path.to_str().unwrap());

        let prop_save: PropSave = serde_json::from_str(&prop_json).unwrap();

        prop_save.name
    }

    pub fn use_item(&mut self, quantity: &mut u32, ctx: &mut ClientTickContext, space: &mut Space, props: &mut Vec<Prop>) {
        *quantity -= 1;

        let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

        let prop = self.to_prop(mouse_pos.into(), ctx.prefabs, space);

        props.push(prop);
    }

    pub fn get_preview_resolution(&self, size: f32, prefabs: &Prefabs, textures: &TextureLoader) -> Vec2 {

        
        let prop_save: PropSave = serde_json::from_str(&prefabs.get_prefab_data(&self.prefab_path.to_string_lossy())).unwrap();

        get_preview_resolution(size, textures, &prop_save.sprite_path)
    }

    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, prefabs: &Prefabs, color: Option<Color>, rotation: f32) {

        let prop_save: PropSave = serde_json::from_str(&prefabs.get_prefab_data(&self.prefab_path.to_string_lossy())).unwrap();
        
        draw_preview(textures, size, draw_pos, color, rotation, &prop_save.sprite_path);
    }

    // might want to change this to not use prefabs
    pub fn to_prop(&self, pos: Isometry2<f32>, prefabs: &Prefabs, space: &mut Space) -> Prop {

        let prop_save: PropSave = serde_json::from_str(&prefabs.get_prefab_data(&self.prefab_path.to_string_lossy())).unwrap();

        Prop::from_save(prop_save, space)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PropItemSave {
    prefab_path: String
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


        // we must manually send a velocity update here because we are despawning the prop here and it wont get automically updated in the tick method
        ctx.network_io.send_network_packet(
            crate::updates::NetworkPacket::PropVelocityUpdate(
                PropVelocityUpdate {
                    velocity: *rigid_body.vels(),
                    id: self.id,
                    area_id,
                }
            )
        );
        
        // if health < 0 {}
        self.dissolve(ctx.textures, space, dissolved_pixels, Some(ctx), area_id);

        

        self.despawn(space, area_id, Some(ctx));
    }
    
    pub fn despawn(&mut self, space: &mut Space, area_id: AreaId, ctx: Option<&mut ClientTickContext>) {
        space.rigid_body_set.remove(self.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);

        self.despawn = true;

        // i dont know if this is a good pattern
        if let Some(ctx) = ctx {
            ctx.network_io.send_network_packet(NetworkPacket::RemovePropUpdate(
            RemovePropUpdate {
                prop_id: self.id,
                area_id: area_id,
            }
            ));
        }
        


    }
    pub fn from_prefab(prefab_path: String, space: &mut Space) -> Self {

        let prop_save: PropSave = serde_json::from_str(&read_to_string(prefab_path).unwrap()).unwrap();

        let prop = Prop::from_save(prop_save, space);

        prop
    }
    pub fn server_tick(&mut self, space: &mut Space, area_id: AreaId, server_io: &mut ServerIO) {

    }

    pub fn dissolve(&mut self, textures: &TextureLoader, space: &mut Space, dissolved_pixels: &mut Vec<DissolvedPixel>, ctx: Option<&mut ClientTickContext>, area_id: AreaId) {

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

        if let Some(ctx) = ctx {
            ctx.network_io.send_network_packet(
                NetworkPacket::DissolveProp(
                    DissolveProp { prop_id: self.id, area_id: area_id }
                )
            );
        }
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

        let current_position = space.rigid_body_set.get(self.rigid_body_handle).unwrap().position();

        if self.last_pos_update.elapsed().as_secs() > 3 {
            ctx.network_io.send_network_packet(
                NetworkPacket::PropPositionUpdate(
                    PropPositionUpdate {
                        area_id,
                        pos: *current_position,
                        prop_id: self.id,
                    }
                )
            );

        }
        

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
                // .ccd_enabled(true)
                // .soft_ccd_prediction(20.)
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
            despawn: false,
            last_pos_update: web_time::Instant::now(),
            name: save.name
            
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
            material: self.material,
            name: self.name.clone()
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
    pub material: PropMaterial,
    #[serde(default = "default_prop_name")]
    name: String
}


fn default_prop_name() -> String {
    "Unnamed Prop".to_string()
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropPositionUpdate {
    pub area_id: AreaId,
    pub pos: Isometry2<f32>,
    pub prop_id: PropId
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemovePropUpdate {
    pub prop_id: PropId,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DissolveProp {
    pub prop_id: PropId,
    pub area_id: AreaId
}



