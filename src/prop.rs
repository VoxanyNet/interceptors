use std::{fs::read_to_string, path::PathBuf};

use async_trait::async_trait;
use glamx::{IVec2, Pose2};
use image::{GenericImageView, Pixel};
use macroquad::{audio::play_sound_once, camera::{Camera2D, set_camera}, color::{BLACK, Color, RED, WHITE}, math::{Rect, Vec2}, prelude::{MaterialParams, gl_use_default_material, gl_use_material, load_material}, shapes::{draw_circle, draw_rectangle}, texture::{DrawTextureParams, RenderTarget, Texture2D, draw_texture_ex, render_target}, window::clear_background};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyVelocity};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};
use crate::{ClientTickContext, Owner, Prefabs, TextureLoader, TickContext, area::AreaId, dissolved_pixel::DissolvedPixel, draw_preview, drawable::Drawable, editor_context_menu::{EditorContextMenu, EditorContextMenuData}, get_preview_resolution, rapier_to_macroquad, space::Space, texture_loader::ClientTextureLoader, updates::NetworkPacket, uuid_u64, weapons::bullet_impact_data::BulletImpactData};


pub const DESTRUCTION_MASK_FRAGMENT_SHADER: &'static str = r#"
#version 100
precision lowp float;

varying vec2 uv;
varying vec4 color;

uniform sampler2D Texture;
uniform sampler2D Mask;

void main() {
    vec4 res = texture2D(Texture, uv);
    vec4 mask = texture2D(Mask, uv);

    // If the mask pixel is dark, don't draw this pixel
    if (mask.r < 0.5) {
        discard;
    }

    gl_FragColor = res * color;
}
"#; 

pub const DESTRUCTION_MASK_VERTEXT_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, EnumIter, Display)]
pub enum Material {
    Wood,
    #[default]
    None
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    sprite_path: PathBuf,
    previous_velocity: RigidBodyVelocity<f32>,
    material: Material,
    pub id: PropId,
    pub owner: Option<Owner>,
    last_sound_play: web_time::Instant,
    pub despawn: bool,
    last_pos_update: web_time::Instant,
    name: String,
    context_menu_data: Option<EditorContextMenuData>,
    layer: u32,
    voxels_modified: bool,
    pub scale: f32,
    pub shader_material: Option<macroquad::material::Material>,
    pub mask: Option<RenderTarget>,
    pub removed_voxels: Vec<glamx::IVec2> 
}

// need to skip the mask render target in partialeq
impl PartialEq for Prop {
    fn eq(&self, other: &Self) -> bool {
        self.rigid_body_handle == other.rigid_body_handle && self.collider_handle == other.collider_handle && self.sprite_path == other.sprite_path && self.previous_velocity == other.previous_velocity && self.material == other.material && self.id == other.id && self.owner == other.owner && self.last_sound_play == other.last_sound_play && self.despawn == other.despawn && self.last_pos_update == other.last_pos_update && self.name == other.name && self.context_menu_data == other.context_menu_data && self.layer == other.layer && self.voxels_modified == other.voxels_modified && self.scale == other.scale && self.shader_material == other.shader_material 
    }
}


impl Prop {

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn draw_preview(&self, textures: &ClientTextureLoader, size: f32, draw_pos: Vec2, _prefabs: &Prefabs, color: Option<Color>, rotation: f32) {
        draw_preview(textures, size, draw_pos, color, rotation, &self.sprite_path);
    }

    pub fn get_preview_resolution(&self, size: f32, _prefabs: &Prefabs, textures: &ClientTextureLoader) -> Vec2 {

        get_preview_resolution(size, textures, &self.sprite_path)
    }

    pub fn handle_bullet_impact(
        &mut self, 
        ctx: &mut TickContext, 
        impact: &BulletImpactData, 
        space: &mut Space, 
        area_id: AreaId,
        _dissolved_pixels: &mut Vec<DissolvedPixel>
    ) {

        if self.despawn {
            return;
        }

        let rigid_body = space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap();
        let _collider = space.collider_set.get_mut(self.collider_handle).unwrap();

        rigid_body.apply_impulse(
            glamx::Vec2::new(impact.bullet_vector.x * 5000., impact.bullet_vector.y * 5000.), 
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
        if let TickContext::Client(_ctx) = ctx {
            //self.dissolve(ctx.textures, space, dissolved_pixels, Some(ctx), area_id);

            log::debug!("epic");
            let impacted_voxels = self.get_voxel_world_positions(space)
                .filter_map(
                    |(voxel_grid_coords, voxel_world_pos)|
                    {
                        if voxel_grid_coords.y == 0 {
                            
                        }

                        //log::debug!("VOXEL WORLD POS: {}, VOXEL GRID COORDS: {:?}, IMPACT_POS: {:?}", voxel_world_pos, voxel_grid_coords, impact.intersection_point);

                        if (voxel_world_pos - impact.intersection_point).length().abs() > 30. {
                            
                            return None
                        }

                        Some(voxel_grid_coords)
                    }
                );

            log::debug!("gamer");
            
            

            let mut impacted_voxels_vec: Vec<IVec2> = impacted_voxels.collect();

            let collider = space.collider_set.get_mut(self.collider_handle).unwrap();
            for voxel in &impacted_voxels_vec {
                collider.shape_mut().as_voxels_mut().unwrap().set_voxel(*voxel, false);
            }
            

            

            self.removed_voxels.append(&mut impacted_voxels_vec);
            self.removed_voxels.dedup();
           //self.removed_voxels = impacted_voxels_vec.clone();
            
            //log::debug!("Impacted voxels: {:?}", impacted_voxels_vec);
        }
        
        //self.mark_despawn();

        // let packet = RemovePropUpdate {
        //     prop_id: self.id,
        //     area_id,
        // }.into();

        // match ctx {
        //     TickContext::Client(ctx) => {
        //         ctx.network_io.send_network_packet(packet);
        //     },
        //     TickContext::Server(ctx) => {
        //         ctx.network_io.send_all_clients(packet);
        //     },
        // };
    
    }

    
    pub fn despawn_callback(&mut self, space: &mut Space, _area_id: AreaId) {
        space.rigid_body_set.remove(self.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }
    pub fn from_prefab(prefab_path: String, space: &mut Space, textures: TextureLoader) -> Self {

        #[cfg(target_os = "linux")]
        let prefab_path = prefab_path.replace("\\", "/");

        let prop_save: PropSave = serde_json::from_str(&read_to_string(prefab_path.to_string()).unwrap()).unwrap();

        let prop = Prop::from_save(prop_save, space, textures);

        prop
    }

    pub fn dissolve(
        &mut self, 
        textures: &ClientTextureLoader, 
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

                let translation = glamx::Vec2::new(
                ((body_translation.x + (x as f32 * x_scale)) - half_extents.x) + 2., 
                ((body_translation.y - (y as f32 * y_scale)) + half_extents.y) - 2.    
                );

                let position = Pose2::new(
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
        _dissolved_pixels: &mut Vec<DissolvedPixel>
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
    pub fn set_pos(
        &mut self, 
        position: glamx::Pose2, 
        space: &mut Space
    ) {
        space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap()
        .set_position(
            position,
            true
        );
    }

    pub fn set_velocity(&mut self, velocity: RigidBodyVelocity<f32>, space: &mut Space) {

        if self.despawn {
            return;
        }
        
        space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap().set_vels(velocity, true);
    }

    pub fn from_save(save: PropSave, space: &mut Space, textures: TextureLoader) -> Self {

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .pose(save.pos)
                
                //.ccd_enabled(true)
                // .soft_ccd_prediction(20.)
        );


        // let collider = space.collider_set.insert_with_parent(
        //     ColliderBuilder::cuboid(save.size.x / 2., save.size.y / 2.)
        //         .mass(save.mass),
        //     body,
        //     &mut space.rigid_body_set
        // );
        

        // this is so amazingly horrible for such a stupid reason i must make it better
        let voxels = match textures {
            TextureLoader::Client(client_texture_loader) => {

                let image = client_texture_loader.get(&save.sprite_path).get_texture_data();

                match &save.voxels {
                    Some(voxels) => voxels.clone(),
                    None => {

                        let mut voxels: Vec<IVec2> = Vec::new();

                        for x in (0..image.width() as u32).step_by(4)  {
                            for y in (0..image.height() as u32).step_by(4)  {
                                let color = image.get_pixel(x, y);

                                if color.a > 0. {
                                    
                                    voxels.push(IVec2::new(x as i32 / 4, y as i32 / 4));
                                } else {
                                    log::debug!("Skipping")
                                }
                            }
                        }

                        voxels
                    },
                }
            },
            TextureLoader::Server(server_texture_loader) =>  {
                let image = server_texture_loader.get(&save.sprite_path);

                match &save.voxels {
                    Some(voxels) => voxels.clone(),
                    None => {

                        let mut voxels: Vec<IVec2> = Vec::new();

                        for x in (0..image.width() as u32).step_by(4)  {
                            for y in (0..image.height() as u32).step_by(4)  {
                                let color = image.get_pixel(x, y);

                                if color.alpha() != 0 {
                                    voxels.push(IVec2::new(x as i32 / 4, y as i32 / 4));
                                }
                            }
                        }

                        voxels
                    },
                }
            },
        };

        log::debug!("number of voxels {:?}", voxels.len());

        

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::voxels(
                glamx::Vec2::new(save.scale * 4., save.scale * 4.,),
                &voxels
            ), 
            body, 
            &mut space.rigid_body_set
        );

        let id = match save.id {
            Some(id) => id,
            None => PropId::new(),
        };


        Self {
            rigid_body_handle: body,
            collider_handle,
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
            layer: save.layer,
            voxels_modified: save.voxels.is_some(),
            scale: save.scale,
            mask: None,
            shader_material: None,
            removed_voxels: vec![]

        }
    }

    pub fn save(&self, space: &Space) -> PropSave {

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let pos = body.position().clone();
        
        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let mass = collider.mass();

        let voxels = if self.voxels_modified {
            let coords: Vec<IVec2> = collider
                .shape()
                .as_voxels().unwrap()
                .voxels()
                .map(|v| v.grid_coords)
                .collect();
            
            coords.into()
        } else {
            None
        };

        PropSave {
            pos,
            mass,
            sprite_path: self.sprite_path.clone(),
            id: Some(self.id.clone()),
            owner: self.owner,
            material: self.material,
            name: self.name.clone(),
            layer: self.layer,
            voxels,
            scale: self.scale
        }
    }

    fn draw_mask(
        &mut self, 
        draw_context: &crate::drawable::DrawContext,
        texture: &Texture2D
    ) {

        if self.shader_material.is_none() {
            self.shader_material = Some(
                load_material(
                    macroquad::prelude::ShaderSource::Glsl 
                    { 
                        vertex: DESTRUCTION_MASK_VERTEXT_SHADER, 
                        fragment: DESTRUCTION_MASK_FRAGMENT_SHADER 
                    }, 
                    MaterialParams { 
                        uniforms: vec![], 
                        textures: vec!["Mask".to_string()],
                        ..Default::default()
                    }
                ).unwrap()
            )
        }

        //log::debug!("Texture width: {:?}, height: {:?}", texture.width(), texture.height());
        
        if self.mask.is_none() {
            self.mask = Some(
                render_target(texture.width() as u32, texture.height() as u32)
            )    
        }

        let mask = self.mask.as_mut().unwrap();
        
        let _then = web_time::Instant::now();
        let mut camera = Camera2D::from_display_rect(
            Rect::new(0., 0., mask.texture.width(), mask.texture.height())
        );  

        camera.render_target = Some(mask.clone());
        camera.zoom.y = -camera.zoom.y;


        set_camera(&camera);

        // clear the mask
        clear_background(WHITE);
        
        let _voxel_size = draw_context.space.collider_set.get(self.collider_handle).unwrap().shape().as_voxels().unwrap().voxel_size();


        for removed_voxel in &self.removed_voxels {
            draw_rectangle(
                removed_voxel.x as f32 * 4., // we dont multiply by the scale here because the texture is scaled when it is drawn!
                (((removed_voxel.y as f32 * 4.) * -1.) + texture.height()) - 4., // need to convert to macroquad coords. THIS -4 COSTED ME HOURS
                4., 
                4., 
                BLACK
            );

            // if removed_voxel.y == 1 {
            //     log::debug!("GAMING: {}", ((removed_voxel.y as f32 * 4.) * -1.) + texture.height());
            // }
            
        }

        //draw_rectangle(10., 0., 10., 10., BLACK);

        set_camera(draw_context.default_camera);
    } 

    fn get_voxel_world_positions(
        &self, 
        space: &Space
    ) -> impl Iterator<Item = (glamx::IVec2, glamx::Vec2)> {
        let collider = space.collider_set.get(self.collider_handle).unwrap();

        let cos = collider.rotation().cos();
        let sin = collider.rotation().sin();

        collider.shape().as_voxels().unwrap().voxels()
            .map(
                move |voxel| 
                {
                    let rotated_x = voxel.center.x * cos - voxel.center.y * sin;
                    let rotated_y = voxel.center.x * sin + voxel.center.y * cos;

                    let world_x = rotated_x + collider.translation().x;
                    let world_y = rotated_y + collider.translation().y;

                    (voxel.grid_coords, glamx::Vec2::new(world_x, world_y))
                }
            )
    }

}

impl EditorContextMenu for Prop {
    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {
        let space = space.unwrap();

        let pos = space.rigid_body_set.get(self.rigid_body_handle).unwrap().translation();
        let size = space.collider_set.get(self.collider_handle).unwrap().shape().as_cuboid().unwrap().half_extents;

        let mpos = rapier_to_macroquad(pos);

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

        *self = Self::from_save(prop_save, ctx.space, ctx.textures.into());
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

        let texture = draw_context.textures.get(&self.sprite_path);

        self.draw_mask(draw_context, texture);

        let mask = self.mask.as_ref().unwrap();
        let material = self.shader_material.as_ref().unwrap();
        material.set_texture("Mask", mask.texture.clone());

        let body = draw_context.space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let _collider = draw_context.space.collider_set.get(self.collider_handle).unwrap();


        //let center_of_mass_macroquad_pos = rapier_to_macroquad(body.center_of_mass());
        let macroquad_pos = rapier_to_macroquad(body.translation());
        
        

        let size = Vec2::new(texture.width() * self.scale, texture.height() * self.scale);

        let pivot = Vec2::new(size.x , size.y);

        gl_use_material(material);
        draw_texture_ex(
            texture, 
            macroquad_pos.x, 
            macroquad_pos.y - pivot.y,
            WHITE,
            DrawTextureParams { 
                dest_size: Some(size), 
                rotation: body.rotation().angle() * -1., 
                pivot: Some(macroquad_pos),
                ..Default::default()
            }
        
        );
        gl_use_default_material();

        draw_circle(macroquad_pos.x, macroquad_pos.y, 2., RED);

        // let mut color = WHITE;
        // color.a = 0.5;
        // for voxel in self.get_voxel_world_positions(draw_context.space) {

        //     let macroquad_pos = rapier_to_macroquad(voxel.1);
        //     draw_rectangle(macroquad_pos.x - (2. * self.scale), macroquad_pos.y - (2. * self.scale), 4. * self.scale, 4. * self.scale, color);
        // }
        
    }

    

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PropSave {
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
    pub voxels: Option<Vec<glamx::IVec2>>
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

// this SHOULD be a temporary fix to make dissolved pixels react to bullets
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StupidDissolvedPixelVelocityUpdate {
    pub area_id: AreaId,
    pub bullet_vector: glamx::Vec2,
    pub weapon_pos: glamx::Vec2
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropVelocityUpdate {
    pub velocity: RigidBodyVelocity<f32>,
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
    pub pos: Pose2,
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



