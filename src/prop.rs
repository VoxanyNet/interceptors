use std::{fs::read_to_string, path::PathBuf};

use async_trait::async_trait;
use macroquad::{audio::play_sound_once, color::Color, math::{Rect, Vec2}, shapes::{DrawRectangleParams, draw_rectangle_ex}};
use nalgebra::{Isometry2, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};

use crate::{ClientId, ClientTickContext, Owner, Prefabs, ServerIO, TickContext, area::AreaId, draw_preview, draw_texture_onto_physics_body, drawable::{DrawContext, Drawable}, editor_context_menu::{EditorContextMenu, EditorContextMenuData}, get_preview_resolution, player::PlayerId, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, weapons::bullet_impact_data::BulletImpactData};

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, EnumIter, Display)]
pub enum PropMaterial {
    Wood,
    #[default]
    None
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq)]
pub struct DissolvedPixelId {
    id: u64
}

impl DissolvedPixelId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}

pub struct DissolvedPixel {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
    color: Color,
    scale: f32,
    spawned: web_time::Instant,
    pub despawn: bool
}


impl DissolvedPixel {

    pub fn tick(&mut self) {

        if self.despawn {
            return;
        }

        let elapsed = self.spawned.elapsed().as_secs_f32();
        
        if elapsed == 0. {
            return;
        }


        self.color.a -= 0.01 * elapsed;

        if self.color.a <= 0. {
            self.mark_despawn();
        }

    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }
    pub fn despawn_callback(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

    pub fn new(
        pos: Isometry2<f32>, 
        space: &mut Space,
        color: Color,
        scale: f32,
        mass: Option<f32>,
        velocity: Option<RigidBodyVelocity>,
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
                .angvel(velocity.angvel)
                .linvel(velocity.linvel)
        );

        let collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(2., 2.).mass(mass),
            rigid_body,
            &mut space.rigid_body_set
        );

        Self {
            body: rigid_body,
            collider,
            color,
            scale,
            spawned: web_time::Instant::now(),
            despawn: false
        }
    }
}



#[async_trait]
impl Drawable for DissolvedPixel {
    async fn draw(&mut self, draw_context: &DrawContext) {
        if self.despawn {
            return;
        }

        let body = draw_context.space.rigid_body_set.get(self.body).unwrap();

        let macroquad_pos = rapier_to_macroquad(*body.translation());

        let shape = draw_context.space.collider_set.get(self.collider).unwrap().shape().as_cuboid().unwrap();


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

    fn draw_layer(&self) -> u32 {
        1
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Prop {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    sprite_path: PathBuf,
    previous_velocity: RigidBodyVelocity,
    material: PropMaterial,
    pub id: PropId,
    pub owner: Option<Owner>,
    last_sound_play: web_time::Instant,
    pub despawn: bool,
    last_pos_update: web_time::Instant,
    name: String,
    context_menu_data: Option<EditorContextMenuData>,
    layer: u32
}


impl Prop {

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, prefabs: &Prefabs, color: Option<Color>, rotation: f32) {
        draw_preview(textures, size, draw_pos, color, rotation, &self.sprite_path);
    }

    pub fn get_preview_resolution(&self, size: f32, prefabs: &Prefabs, textures: &TextureLoader) -> Vec2 {

        get_preview_resolution(size, textures, &self.sprite_path)
    }

    pub fn handle_bullet_impact(
        &mut self, 
        ctx: &mut TickContext, 
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
                if ctx.id() != owner {
                    let packet = crate::updates::NetworkPacket::PropVelocityUpdate(
                        PropVelocityUpdate {
                            velocity: *rigid_body.vels(),
                            id: self.id,
                            area_id,
                        }
                    );

                    match ctx {
                        TickContext::Client(ctx) => {
                            ctx.network_io.send_network_packet(packet);
                        },
                        TickContext::Server(ctx) => {
                            ctx.network_io.send_all_clients(packet);
                        },
                    };

                }
            },
            None => {},
        }

        // we must manually send a velocity update here because we are despawning the prop here and it wont get automatically updated in the tick method
        let packet = crate::updates::NetworkPacket::PropVelocityUpdate(
            PropVelocityUpdate {
                velocity: *rigid_body.vels(),
                id: self.id,
                area_id,
            }
        );

        match ctx {
            TickContext::Client(ctx) => {
                ctx.network_io.send_network_packet(packet);
            },
            TickContext::Server(ctx) => {
                ctx.network_io.send_all_clients(packet);
            },
        };
        
        // if health < 0 {}

        // Server cannot dissolve props right now but we arent even going to do that so im going to worry about it
        if let TickContext::Client(ctx) = ctx {
            self.dissolve(ctx.textures, space, dissolved_pixels, Some(ctx), area_id);
        }
        
        self.mark_despawn();

        let packet = RemovePropUpdate {
            prop_id: self.id,
            area_id,
        }.into();

        match ctx {
            TickContext::Client(ctx) => {
                ctx.network_io.send_network_packet(packet);
            },
            TickContext::Server(ctx) => {
                ctx.network_io.send_all_clients(packet);
            },
        };
    
    }

    
    pub fn despawn_callback(&mut self, space: &mut Space, area_id: AreaId) {
        space.rigid_body_set.remove(self.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }
    pub fn from_prefab(prefab_path: String, space: &mut Space) -> Self {

        #[cfg(target_os = "linux")]
        let prefab_path = prefab_path.replace("\\", "/");

        let prop_save: PropSave = serde_json::from_str(&read_to_string(prefab_path.to_string()).unwrap()).unwrap();

        let prop = Prop::from_save(prop_save, space);

        prop
    }
    pub fn server_tick(&mut self, space: &mut Space, area_id: AreaId, server_io: &mut ServerIO) {

    }

    pub fn dissolve(
        &mut self, 
        textures: &TextureLoader, 
        space: &mut Space, 
        dissolved_pixels: &mut Vec<DissolvedPixel>, 
        ctx: Option<&mut ClientTickContext>, 
        area_id: AreaId
    ) {

        let collider = space.collider_set.get(self.collider_handle).unwrap().clone();
        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap().clone();
        let body_translation = body.translation();
        let texture = textures.get(&self.sprite_path);
        let half_extents = collider.shape().as_cuboid().unwrap().half_extents;
        let x_scale = (half_extents.x * 2.) / texture.width() ;
        let y_scale = (half_extents.y * 2.) / texture.height();
        let texture_data = texture.get_texture_data();
        let total_pixel_count = texture.width() * texture.height();

        for x in (0..texture.width() as u32).step_by(4) {
            for y in (0..texture.height() as u32).step_by(4) {
                // create an average of the 4 neighboring pixels
                // start with bottom left
                let mut color = texture_data.get_pixel(x, y);
                let mut pixel_count = 1;
                // bottom right
                if x + 1 <= texture.width() as u32 {
                    let bottom_right_color = texture_data.get_pixel(x + 1, y);

                    color.r += bottom_right_color.r;
                    color.g += bottom_right_color.g;
                    color.b += bottom_right_color.b;

                    pixel_count += 1;
                }

                // top left
                if y + 1 <= texture.height() as u32 {
                    let top_left_color = texture_data.get_pixel(x, y + 1);

                    color.r += top_left_color.r;
                    color.g += top_left_color.g;
                    color.b += top_left_color.b;

                    pixel_count += 1;
                }

                // top right
                if x + 1 <= texture.width() as u32 && y + 1 <= texture.height() as u32 {
                    let top_right_color = texture_data.get_pixel(x + 1, y + 1);

                    color.r += top_right_color.r;
                    color.g += top_right_color.g;
                    color.b += top_right_color.b;

                    pixel_count += 1;

                }

                color.r /= pixel_count as f32;
                color.g /= pixel_count as f32;
                color.b /= pixel_count as f32;

                let translation = Vector2::new(
                ((body_translation.x + (x as f32 * x_scale)) - half_extents.x) + 2., 
                ((body_translation.y - (y as f32 * y_scale)) + half_extents.y) - 2.    
                );

                let position = Isometry2::new(
                    translation, 
                    body.rotation().angle()
                );

                
                dissolved_pixels.push(
                    DissolvedPixel::new(
                        position, 
                        space, 
                        color, 
                        x_scale, 
                        Some(collider.mass() / total_pixel_count), 
                        Some(*body.vels()),
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

                if contact_pair.total_impulse_magnitude() > 25000. && self.last_sound_play.elapsed().as_secs() > 1 {

                    
                    
                    play_sound_once(ctx.sounds.get(PathBuf::from("assets\\sounds\\crate\\tap.wav")));

                    self.last_sound_play = web_time::Instant::now();
                }
            }
        }
    }

    pub fn owner_tick(
        &mut self, 
        ctx: &mut TickContext, 
        space: &mut Space, 
        area_id: AreaId, 
        dissolved_pixels: &mut Vec<DissolvedPixel>
    ) {

        if let TickContext::Client(ctx) = ctx {
            self.play_impact_sound(space, ctx);
        }
        

        let current_velocity = *space.rigid_body_set.get(self.rigid_body_handle).unwrap().vels();
        let current_position = space.rigid_body_set.get(self.rigid_body_handle).unwrap().position();

        if self.last_pos_update.elapsed().as_secs() > 3 {

            let packet = NetworkPacket::PropPositionUpdate(
                PropPositionUpdate {
                    area_id,
                    pos: *current_position,
                    prop_id: self.id,
                }
            );

            match ctx {
                TickContext::Client(ctx) => {
                    ctx.network_io.send_network_packet(packet);
                },
                TickContext::Server(ctx) => {
                    ctx.network_io.send_all_clients(packet);
                },
            }


        }
        

        // need to add a cooldown?
        if current_velocity != self.previous_velocity {

            let packet = NetworkPacket::PropVelocityUpdate(
                PropVelocityUpdate {
                    velocity: current_velocity,
                    id: self.id,
                    area_id: area_id
                }
            );
            
            match ctx {
                TickContext::Client(ctx) => {
                    ctx.network_io.send_network_packet(packet);
                },
                TickContext::Server(ctx) => {
                    ctx.network_io.send_all_clients(packet);
                },
            };
        }
    }   

    pub fn tick(
        &mut self, 
        space: &mut Space, 
        area_id: AreaId, 
        ctx: &mut TickContext, 
        dissolved_pixels: &mut Vec<DissolvedPixel>
    ) {

        if self.despawn {
            return;
        }

        if let Some(owner) = self.owner {
            if owner == ctx.id() {
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

        if self.despawn {
            return;
        }
        
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
            name: save.name,
            context_menu_data: None,
            layer: save.layer

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
            name: self.name.clone(),
            layer: self.layer
        }
    }

}

impl EditorContextMenu for Prop {
    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {
        let space = space.unwrap();

        let pos = space.rigid_body_set.get(self.rigid_body_handle).unwrap().translation();
        let size = space.collider_set.get(self.collider_handle).unwrap().shape().as_cuboid().unwrap().half_extents;

        let mpos = rapier_to_macroquad(*pos);

        Rect::new(mpos.x - size.x, mpos.y - size.y, size.x * 2., size.y * 2.)

    }

    fn context_menu_data_mut(&mut self) -> &mut Option<crate::editor_context_menu::EditorContextMenuData> {
        &mut self.context_menu_data
    }

    fn context_menu_data(&self) -> &Option<crate::editor_context_menu::EditorContextMenuData> {
        &self.context_menu_data
    }

    fn despawn(&mut self) -> Option<&mut bool> {
        Some(&mut self.despawn)
    }

    fn data_editor_export(&self, ctx: &crate::editor_context_menu::DataEditorContext) -> Option<String> {
        serde_json::to_string_pretty(
            &self.save(ctx.space)
        ).unwrap().into()
    }

    fn data_editor_import(&mut self, json: String, ctx: &mut crate::editor_context_menu::DataEditorContext) {
        let prop_save: PropSave = serde_json::from_str(&json).unwrap();

        *self = Self::from_save(prop_save, ctx.space);
    }

    fn layer(&mut self) -> Option<&mut u32> {
        Some(&mut self.layer)
    }




    
}
#[async_trait]
impl Drawable for Prop {
    async fn draw(&mut self, draw_context: &crate::drawable::DrawContext) {
         if self.despawn {
            return;
        }
        draw_texture_onto_physics_body(
            self.rigid_body_handle, 
            self.collider_handle, 
            draw_context.space, 
            &self.sprite_path, 
             draw_context.textures, 
            false, 
            false, 
            0.
        ).await;
    }

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}

// this SHOULD be a temporary fix to make dissolved pixels react to bullets
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StupidDissolvedPixelVelocityUpdate {
    pub area_id: AreaId,
    pub bullet_vector: Vector2<f32>,
    pub weapon_pos: Vector2<f32>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PropSave {
    pub size: Vec2,
    pub pos: Isometry2<f32>,
    pub mass: f32,
    pub sprite_path: PathBuf,
    pub id: Option<PropId>,
    #[serde(default)]
    pub owner: Option<Owner>,
    #[serde(default)]
    pub material: PropMaterial,
    #[serde(default = "default_prop_name")]
    pub name: String,
    #[serde(default)]
    pub layer: u32
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
    pub owner: Option<Owner>,
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



