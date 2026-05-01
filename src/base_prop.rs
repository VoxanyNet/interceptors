
use std::{collections::HashSet, fs::read_to_string, path::PathBuf};

use async_trait::async_trait;
use glamx::{IVec2, Pose2, vec2};
use image::{GenericImageView, Pixel};
use macroquad::{audio::play_sound_once, camera::{Camera2D, set_camera}, color::{BLACK, BLUE, Color, GREEN, RED, VIOLET, WHITE}, input::{KeyCode, is_key_pressed}, math::{Rect, Vec2}, prelude::{MaterialParams, gl_use_default_material, gl_use_material, load_material}, shapes::{draw_circle, draw_rectangle}, text::{TextParams, draw_text, draw_text_ex}, texture::{DrawTextureParams, RenderTarget, Texture2D, draw_texture_ex, render_target}, window::clear_background};
use rapier2d::prelude::{AxisMask, ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyType, RigidBodyVelocity, SharedShape, VoxelData};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};
use crate::{ClearBackgroundParameters, ClientId, ClientTickContext, DrawCommand, DrawRectangleParameters, DrawTextureParameters, Owner, Prefabs, SetCameraParameters, SetMaterialTextureParameters, TextureLoader, TickContext, UseMaterialParameters, area::{Area, AreaContext, AreaId}, base_prop_save::BasePropSave, dissolved_pixel::DissolvedPixel, draw_preview, drawable::Drawable, editor_context_menu::{EditorContextMenu, EditorContextMenuData}, flood_fill, get_preview_resolution, prop::Prop, prop_save::PropSave, rapier_to_macroquad, space::Space, texture_loader::ClientTextureLoader, updates::NetworkPacket, uuid_u64, weapons::bullet_impact_data::BulletImpactData};



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
pub struct BaseProp {
    pub last_ownership_change: web_time::Instant,
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    pub sprite_path: PathBuf,
    pub previous_velocity: RigidBodyVelocity<f32>,
    pub material: Material,
    pub id: PropId,
    pub owner: Option<Owner>,
    pub last_sound_play: web_time::Instant,
    pub despawn: bool,
    pub last_pos_update: web_time::Instant,
    pub last_velocity_update: web_time::Instant,
    pub name: String,
    pub context_menu_data: Option<EditorContextMenuData>,
    pub layer: u32,
    pub rigid_body_type: RigidBodyType, // we store this here as well so we can revert back to it when we regain ownership
    pub voxels_modified: bool,
    pub scale: f32,
    pub shader_material: Option<macroquad::material::Material>,
    pub mask: Option<RenderTarget>,
    pub removed_voxels: Vec<glamx::IVec2>,
    pub spawned: web_time::Instant,
    // how long this prop should live before being automatically despawned
    pub lifespan: Option<web_time::Duration>,
    // should we send network updates to the server
    pub sync_physics: bool,
    pub last_received_position_update: web_time::Instant,
    pub last_sent_position_update: web_time::Instant

}

impl Prop for BaseProp {

    fn layer(&self) -> u32 {
        self.layer
    }

    fn draw(&mut self, ctx: &mut TickContext, space: &mut Space) {

    
        
        if self.despawn {
            return;
        }
        
        // this is majorly stupid
        let (texture, material) = if let TickContext::Client(client_ctx) = ctx {
            let texture = client_ctx.textures.get(&self.sprite_path);
            let material = client_ctx.material_loader.get("materials/destruction");
            (texture, material)
        } else if let TickContext::Editor(editor_ctx) = ctx {

            let texture = editor_ctx.textures.get(&self.sprite_path);
            let material = editor_ctx.material_loader.get("materials/destruction");
            (texture, material)

        } else {
            todo!()
        };


        self.draw_mask(ctx, texture);

        let mask = self.mask.as_ref().unwrap();
        
        ctx.add_draw_command(
            self.layer, 
            DrawCommand::SetMaterialTexture(
                SetMaterialTextureParameters {
                    material: material.clone(),
                    texture_name: "Mask".to_string(),
                    texture: mask.texture.clone(),
                }
            )
        );
        
        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let macroquad_pos = rapier_to_macroquad(body.translation());
        let size = Vec2::new(texture.width() * self.scale, texture.height() * self.scale);
        let pivot = Vec2::new(size.x , size.y);
        let mut color = WHITE;
        if let Some(lifespan) = self.lifespan {
            // if (lifespan - self.spawned.elapsed()).as_secs() <= 5 {

            //     //color.a = (lifespan.as_secs_f32() - self.spawned.elapsed().as_secs_f32()) / 5.
            // }
        }

        ctx.add_draw_command(
            self.layer, 
            DrawCommand::UseMaterial(
                UseMaterialParameters {
                    material: material.clone(),
                }
            )
        );

        //gl_use_material(material);

        ctx.add_draw_command(
            self.layer, 
            DrawCommand::DrawTexture(
                DrawTextureParameters {
                    texture: self.sprite_path.clone(),
                    position: Vec2 { 
                        x: macroquad_pos.x, 
                        y: macroquad_pos.y - pivot.y
                    },
                    color,
                    params: DrawTextureParams {
                        dest_size: Some(size),
                        rotation: body.rotation().angle() * -1.,
                        pivot: Some(macroquad_pos),
                        ..Default::default()
                    },
                }
            )
        );

   


        if let RigidBodyType::KinematicPositionBased = space.rigid_body_set.get(self.rigid_body_handle).unwrap().body_type() {

            let mut color = WHITE;

            color.a = 1. - (self.last_received_position_update.elapsed().as_secs_f32() / 1.);

            color.a = color.a.max(0.4);

            ctx.add_draw_command(
                self.layer, 
                DrawCommand::DrawTexture(
                    DrawTextureParameters {
                        texture: self.sprite_path.clone(),
                        position: Vec2 {
                            x: macroquad_pos.x,
                            y: macroquad_pos.y - pivot.y,
                        },
                        color,
                        params: DrawTextureParams {
                            dest_size: Some(size),
                            rotation: body.rotation().angle() * -1.,
                            pivot: Some(macroquad_pos),
                            ..Default::default()
                        },
                    }
                )
            );
        }

        let mut color = GREEN;

        color.a = 1. - (self.last_sent_position_update.elapsed().as_secs_f32() / 1.);


        // draw_texture_ex(
        //     texture,
        //     macroquad_pos.x,
        //     macroquad_pos.y - pivot.y,
        //     color,
        //     DrawTextureParams {
        //         dest_size: Some(size),
        //         rotation: body.rotation().angle() * -1.,
        //         pivot: Some(macroquad_pos),
        //         ..Default::default()
        //     }

        // );
        // if let Some(owner) = self.owner {
        //     if let Owner::ClientId(owner) = owner {
        //         if owner == draw_context.id {
        //             draw_texture_ex(
        //                 texture,
        //                 macroquad_pos.x,
        //                 macroquad_pos.y - pivot.y,
        //                 RED,
        //                 DrawTextureParams {
        //                     dest_size: Some(size),
        //                     rotation: body.rotation().angle() * -1.,
        //                     pivot: Some(macroquad_pos),
        //                     ..Default::default()
        //                 }

        //             );
        //         }
        //     }
        // }

        //draw_text(&format!("{:?}", self.owner), macroquad_pos.x, macroquad_pos.y, 20., WHITE);



        //gl_use_default_material();

        ctx.add_draw_command(
            self.layer, 
            DrawCommand::UseDefaultMaterial
        );

        let mut color = WHITE;
        color.a = 0.5;



        // for voxel in collider.shape().as_voxels().unwrap().voxels() {

        //     let pos = collider.shape().as_voxels().unwrap().voxel_center(voxel.grid_coords);
        //     let macroquad_pos = rapier_to_macroquad(pos);
        //     draw_rectangle(macroquad_pos.x - (2. * self.scale), macroquad_pos.y - (2. * self.scale), 4. * self.scale, 4. * self.scale, color);

        // }
        // for voxel in self.get_voxel_world_positions(draw_context.space) {

        //     let mut color = match collider.shape().as_voxels().unwrap().voxel_state(voxel.0.grid_coords).unwrap().voxel_type() {
        //         rapier2d::prelude::VoxelType::Empty => RED,
        //         rapier2d::prelude::VoxelType::Vertex => GREEN,
        //         rapier2d::prelude::VoxelType::Face => BLUE,
        //         rapier2d::prelude::VoxelType::Interior => VIOLET,
        //     };

        //     color.a = 0.5;
        //     let macroquad_pos = rapier_to_macroquad(voxel.1);
        //     draw_rectangle(macroquad_pos.x - (4.), macroquad_pos.y - (4.), 8., 8., color);

        // }

    }

    fn set_name(&mut self, name: &str) {
        self.name = name.into()
    }

    fn set_material(&mut self, new_material: Material) {
        self.material = new_material
    }

    fn set_mass(&self, space: &mut Space, new_mass: f32) {
        let collider = space.collider_set.get_mut(self.collider_handle).unwrap();

        collider.set_mass(new_mass);
    }

    fn update_menu(&mut self, space: &mut Space, camera_rect: &Rect, selected: bool, textures: &ClientTextureLoader) {
        <BaseProp as EditorContextMenu>::update_menu(self, space, camera_rect, selected, textures);
    }
    fn name(&self) -> String {
        self.name.clone()
    }

    fn rigid_body_handle(&self) -> RigidBodyHandle {
        self.rigid_body_handle
    }

    fn collider_handle(&self) -> ColliderHandle {
        self.collider_handle
    }

    fn sprite_path(&self) -> PathBuf {
        self.sprite_path.clone()
    }

    fn tick(&mut self, area_context: &mut AreaContext, ctx: &mut TickContext) {
        self.inner_tick(area_context, ctx);
    }

    fn id(&self) -> PropId {
        self.id
    }

    fn should_despawn(&self) -> bool {
        self.despawn
    }

    fn despawn_callback(&mut self, space: &mut Space) {
        self.inner_despawn_callback(space);
    }

    fn handle_bullet_impact(
            &mut self,
            ctx: &mut TickContext,
            area_context: &mut AreaContext,
            impact: &BulletImpactData,
        ) {
        self.inner_handle_bullet_impact(ctx, area_context, impact);
    }

    fn save(&self, space: &Space) -> Box<dyn PropSave> {
        self.inner_save(space).into()
    }
    
    fn last_ownership_change(&self) -> web_time::Instant {
        self.last_ownership_change
    }

    fn owner(&self) -> Option<Owner> {
        self.owner
    }

    fn owner_mut(&mut self) -> &mut Option<Owner> {
        &mut self.owner
    }

    fn last_ownership_change_mut(&mut self) -> &mut web_time::Instant {
        &mut self.last_ownership_change
    }

    fn removed_voxels(&self) -> &Vec<glamx::IVec2> {
        &self.removed_voxels
    }

    fn removed_voxels_mut(&mut self) -> &mut Vec<glamx::IVec2> {
        &mut self.removed_voxels
    }

    fn voxels_modified(&self) -> &bool {
        &self.voxels_modified
    }

    fn voxels_modified_mut(&mut self) -> &mut bool {
        &mut self.voxels_modified
    }

    fn last_received_position_update(&self) -> web_time::Instant {
        self.last_ownership_change
    }

    fn last_received_position_update_mut(&mut self) -> &mut web_time::Instant {
        &mut self.last_ownership_change
    }
    
    fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    fn draw_editor_context_menu(&self) {
        <BaseProp as EditorContextMenu>::draw_editor_context_menu(&self);
    }


    
    
}

// need to skip the mask render target in partialeq
impl PartialEq for BaseProp {
    fn eq(&self, other: &Self) -> bool {
        self.rigid_body_handle == other.rigid_body_handle && self.collider_handle == other.collider_handle && self.sprite_path == other.sprite_path && self.previous_velocity == other.previous_velocity && self.material == other.material && self.id == other.id && self.owner == other.owner && self.last_sound_play == other.last_sound_play && self.despawn == other.despawn && self.last_pos_update == other.last_pos_update && self.name == other.name && self.context_menu_data == other.context_menu_data && self.layer == other.layer && self.voxels_modified == other.voxels_modified && self.scale == other.scale && self.shader_material == other.shader_material
    }
}


impl BaseProp {


    pub fn name(&self) -> String {


        self.name.clone()
    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn force_owner_update_with_networking(
        &mut self,
        new_owner: Owner,
        area_context: &AreaContext,
        ctx: &mut TickContext
    ) {
        self.owner = Some(new_owner);

        self.last_ownership_change = web_time::Instant::now();

        ctx.send_network_packet(
            PropUpdateOwner {
                owner: Some(new_owner),
                id: self.id,
                area_id: *area_context.id,
            }.into()
        );
    }

    pub fn draw_preview(&self, ctx: &mut TickContext, size: f32, draw_pos: Vec2, _prefabs: &Prefabs, color: Option<Color>, rotation: f32) {
        draw_preview(ctx, size, draw_pos, color, rotation, &self.sprite_path, self.layer);
    }

    pub fn get_preview_resolution(&self, size: f32, _prefabs: &Prefabs, textures: &ClientTextureLoader) -> Vec2 {

        get_preview_resolution(size, textures, &self.sprite_path)
    }

    /// Set the prop to kinematic if we dont own it
    pub fn update_kinematic_state(
        &mut self,
        ctx: &TickContext,
        space: &mut Space
    ) {

        let body = space.rigid_body_set.get_mut(self.rigid_body_handle).unwrap();

        if let Some(owner) = self.owner {
            if owner != ctx.id() {


                if body.body_type() != RigidBodyType::KinematicPositionBased {
                    body.set_body_type(RigidBodyType::KinematicPositionBased, true);
                }

            } else {

                if body.body_type() != self.rigid_body_type {


                    body.set_body_type(self.rigid_body_type, true);
                }
            }
        } else {
            // also make kinematic if the prop doesnt have an owner
            // might want to change this in the future or maybe make it so that props must always have owners

            if body.body_type() != RigidBodyType::KinematicPositionBased {
                body.set_body_type(RigidBodyType::KinematicPositionBased, true);
            }

        }
    }

    pub fn break_apart(
        &mut self,
        area_id: AreaId,
        ctx: &mut TickContext,
        space: &mut Space,
        impacted_voxels: &Vec<glamx::IVec2>,
        props: &mut Vec<Box<dyn Prop>>
    ) -> Option<Vec<IVec2>> {
        let voxels = space.collider_set
            .get_mut(self.collider_handle)
            .unwrap()
            .shape_mut()
            .as_voxels_mut()
            .unwrap();

        let mut potential_island_seeds: HashSet<glamx::IVec2> = HashSet::new();
        let mut islands: Vec<HashSet<glamx::IVec2>> = Vec::new();
        let mut global_visited_voxels: HashSet<glamx::IVec2> = HashSet::new();

        for voxel in impacted_voxels {
            let neighbors = [
                glamx::IVec2 { x: voxel.x + 1, y: voxel.y},
                glamx::IVec2 { x: voxel.x - 1, y: voxel.y},
                glamx::IVec2 { x: voxel.x, y: voxel.y + 1},
                glamx::IVec2 { x: voxel.x, y: voxel.y - 1}
            ];

            for neighbor in neighbors {

                if let Some(voxel_state) = voxels.voxel_state(neighbor) {
                    if !voxel_state.is_empty() {
                        potential_island_seeds.insert(neighbor);
                    }
                }

            }
        }

        for (idx, seed) in potential_island_seeds
            .iter()
            .enumerate() {

            if global_visited_voxels.contains(&seed) {
                continue;
            }

            let new_island = flood_fill(
                *seed,
                &voxels
            );

            for voxel in &new_island {
                global_visited_voxels.insert(*voxel);
            }

            islands.push(new_island);

        }

        if islands.len() <= 1 {
            return None;
        }

        let new_islands = islands.split_off(1);

        for island in &new_islands {
            let voxels: Vec<glamx::IVec2> = island.iter().cloned().collect();
            let new_prop = Self::fragment_from_existing(
                &self,
                ctx,
                voxels,
                space,
                impacted_voxels,
                None,
                false,
                true
            );

            ctx.send_network_packet(
                NewProp {
                    prop: new_prop.inner_save(space).into(),
                    area_id,
                }.into()
            );

            // when we are breaking apart props here we are creating new base props, not the actual prop. this might cause problems
            props.push(Box::new(new_prop));
        }


        // Determine the new voxel state of this prop

        let mut new_voxels: HashSet<IVec2> = space.collider_set
            .get_mut(self.collider_handle)
            .unwrap()
            .shape_mut()
            .as_voxels_mut()
            .unwrap()
            .voxels()
            .filter_map(
                |x|
                {
                    if !x.state.is_empty() {
                        Some(x.grid_coords)
                    } else {
                        None
                    }
                }
            ).collect();

        for island in new_islands {
            for voxel_index in island {

                // this makes sure we don't double remove an impacted voxel
                if impacted_voxels.contains(&voxel_index) {
                    continue;
                }
                new_voxels.remove(&voxel_index);
                self.removed_voxels.push(voxel_index);
            }
        }

        let new_voxels_vec: Vec<IVec2> = new_voxels.iter().cloned().collect();

        space.collider_set.remove(
            self.collider_handle,
            &mut space.island_manager,
            &mut space.rigid_body_set,
            true
        );


        let new_collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::voxels(vec2(8., 8.), &new_voxels_vec),
            self.rigid_body_handle,
            &mut space.rigid_body_set
        );



        self.collider_handle = new_collider_handle;

        return Some(new_voxels_vec)

    }

    fn get_impacted_voxels(
        &self,
        space: &Space,
        impact: &BulletImpactData
    ) -> Vec<glamx::IVec2> {

        self.get_voxel_world_positions(space)
            .filter(|(_, voxel_world_pos)| {
                (voxel_world_pos - impact.intersection_point).length() < 10.
            })
            .map(|(voxel, _)| voxel.grid_coords)
            .collect()


    }
    fn check_if_no_voxels(&self, space: &Space) -> bool {
        let collider = space.collider_set.get(self.collider_handle).unwrap();

        !collider
            .shape()
            .as_voxels()
            .unwrap()
            .voxels()
            .any(|voxel| !voxel.state.is_empty())
    }

    pub fn inner_handle_bullet_impact(
        &mut self,
        ctx: &mut TickContext,
        area_context: &mut AreaContext,
        impact: &BulletImpactData,
    ) {

        // OPTIMIZATION IDEA
        // dont recreate the shape twice. i dont wanna type the rest out but you'll figure it out

        if self.despawn {return}

        let rigid_body = area_context.space
            .rigid_body_set
            .get_mut(self.rigid_body_handle)
            .unwrap();

        rigid_body.apply_impulse(
            glamx::Vec2::new(
                impact.bullet_vector.x * 5000.,
                impact.bullet_vector.y * 5000.
            ),
            true
        );

        // BECOME the owner if arent the owner!!!!!!!!!!!!! RARGHHHHHH
        if let Some(owner) = self.owner && ctx.id() != owner {
            self.force_owner_update_with_networking(ctx.id(), area_context, ctx);
        }

        // only break apart props on client side FOR NOW
        // let TickContext::Client(_) = ctx else {
        //     return;
        // };

        let mut impacted_voxels = self.get_impacted_voxels(area_context.space, impact);

        // this will probably never be zero
        if impacted_voxels.len() != 0 {
            self.voxels_modified = true;
        }

        let collider_voxels = area_context.space.collider_set
            .get_mut(self.collider_handle)
            .unwrap()
            .shape_mut()
            .as_voxels_mut()
            .unwrap();


        let mut new_voxels: Vec<glamx::IVec2> = collider_voxels.voxels()
            .filter(|voxel| !impacted_voxels.contains(&voxel.grid_coords))
            .map(|voxel| voxel.grid_coords)
            .collect();

        area_context.space.collider_set
            .get_mut(self.collider_handle)
            .unwrap()
            .set_shape(
                SharedShape::voxels(glamx::vec2(8., 8.), &new_voxels)
            );


        // COPY THIS ABOVE??
        if self.check_if_no_voxels(area_context.space) == true {
            self.mark_despawn();
            return;
        }

        if let Some(break_apart_new_voxels) = self.break_apart(*area_context.id, ctx, area_context.space, &impacted_voxels, area_context.props) {

            new_voxels = break_apart_new_voxels;
        }

        self.removed_voxels.append(&mut impacted_voxels);
        self.removed_voxels.dedup();

        ctx.send_network_packet(
            UpdatePropVoxels {
                prop_id: self.id,
                area_id: *area_context.id,
                new_voxels: new_voxels,
                removed_voxels: self.removed_voxels.clone(),
            }.into()
        );
        if impacted_voxels.len() > 0 {


        }

    }


    pub fn inner_despawn_callback(&mut self, space: &mut Space) {
        space.rigid_body_set.remove(self.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }
    pub fn from_prefab(prefab_path: String, space: &mut Space, textures: TextureLoader) -> Self {

        #[cfg(target_os = "linux")]
        let prefab_path = prefab_path.replace("\\", "/");

        let prop_save: BasePropSave = serde_json::from_str(&read_to_string(prefab_path.to_string()).unwrap()).unwrap();

        let prop = BaseProp::from_save(prop_save, space, textures);

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



                    //play_sound_once(ctx.sounds.get(PathBuf::from("assets\\sounds\\crate\\tap.wav")));

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

        let voxels = space.collider_set.get_mut(self.collider_handle).unwrap().shape_mut().as_voxels_mut().unwrap();


        // for voxel in voxels.voxels() {

        // }


        if let TickContext::Client(ctx) = ctx {
            self.play_impact_sound(space, ctx);
        }


        let current_velocity = *space.rigid_body_set.get(self.rigid_body_handle).unwrap().vels();
        let current_position = space.rigid_body_set.get(self.rigid_body_handle).unwrap().position();

        if self.last_pos_update.elapsed().as_millis() > 16
        && self.sync_physics {


            let packet = NetworkPacket::PropPositionUpdate(
                PropPositionUpdate {
                    area_id,
                    pos: *current_position,
                    prop_id: self.id,
                }
            );

            ctx.send_network_packet(packet);
            

            self.last_sent_position_update = web_time::Instant::now();

            self.last_pos_update = web_time::Instant::now();


        }


        if let Some(lifespan) = self.lifespan {
            if self.spawned.elapsed() > lifespan {
                self.mark_despawn();

                ctx.send_network_packet(
                    RemovePropUpdate {
                        prop_id: self.id,
                        area_id,
                    }.into()
                );
            }
        }
    }

    pub fn inner_tick(
        &mut self,
        area_context: &mut AreaContext,
        ctx: &mut TickContext,
    ) {


        if self.despawn {
            return;
        }

        self.update_kinematic_state(ctx, area_context.space);

        if let Some(owner) = self.owner {
            if owner == ctx.id() {
                self.owner_tick(ctx, area_context.space, *area_context.id, area_context.dissolved_pixels);
            }
        }

        


        let current_velocity = *area_context.space.rigid_body_set.get(self.rigid_body_handle).unwrap().vels();

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

    pub fn fragment_from_existing(
        other: &Self,
        ctx: &TickContext,
        fragment_voxels: Vec<glamx::IVec2>,
        space: &mut Space,
        impacted_voxels: &Vec<glamx::IVec2>, // this is for a dumb fix
        lifespan: Option<web_time::Duration>,
        collide_with_players: bool,
        sync_physics: bool
    ) -> Self { 

        let other_body = space.rigid_body_set.get(other.rigid_body_handle).unwrap();

        let body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .pose(*other_body.position())
                .linvel(other_body.linvel())
                .angvel(other_body.angvel())
        );


        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::voxels(vec2(8., 8.), &fragment_voxels),
            body,
            &mut space.rigid_body_set
        );

        let other_collider = space.collider_set.get(other.collider_handle).unwrap();
        let other_voxels = other_collider.shape().as_voxels().unwrap();

        // we need to make sure that all explicitly deleted voxels get transferred over to the new prop but the voxels that were removed in the most recent fire don't get added to the list until after this function is run so we needed to pass in this tick's impacted voxels so we can add it. there is probably a better way to do this but im tired and its z and this will work
        let mut removed_voxels = other.removed_voxels.clone();
        removed_voxels.extend(impacted_voxels);

        for voxel in other_voxels.voxels() {
            // draw over voxels that are part of the other prop
            if !fragment_voxels.contains(&voxel.grid_coords)  {
                removed_voxels.push(voxel.grid_coords);
            }
        }

        removed_voxels.extend(&other.removed_voxels);




        Self {
            last_received_position_update: web_time::Instant::now(),
            last_ownership_change: web_time::Instant::now(),
            rigid_body_type: RigidBodyType::Dynamic, // prop fragmnets are always dynamic
            rigid_body_handle: body,
            collider_handle: collider_handle,
            sprite_path: other.sprite_path.clone(),
            previous_velocity: other.previous_velocity,
            material: other.material,
            id: PropId::new(),
            owner: Some(ctx.id()), // for some reason other does not have an owner when testing
            last_sound_play: other.last_sound_play,
            despawn: false,
            last_pos_update: web_time::Instant::now(),
            last_velocity_update: web_time::Instant::now(),
            name: other.name.clone(),
            context_menu_data: None,
            layer: other.layer,
            voxels_modified: true,
            scale: other.scale,
            shader_material: other.shader_material.clone(),
            mask: None,
            removed_voxels,
            spawned: web_time::Instant::now(),
            lifespan: lifespan,
            sync_physics,
            last_sent_position_update: web_time::Instant::now(),


        }
    }

    pub fn from_save(
        save: BasePropSave,
        space: &mut Space,
        textures: TextureLoader,
    ) -> Self {

        let body_builder = match save.rigid_body_type {
            RigidBodyType::Dynamic => RigidBodyBuilder::dynamic(),
            RigidBodyType::Fixed => RigidBodyBuilder::fixed(),
            RigidBodyType::KinematicPositionBased => RigidBodyBuilder::kinematic_position_based(),
            RigidBodyType::KinematicVelocityBased => RigidBodyBuilder::kinematic_velocity_based()
        };

        let body = space.rigid_body_set.insert(
            body_builder
                .pose(save.pos)
                //.ccd_enabled(true)
                // .soft_ccd_prediction(20.)
        );


        // this is so amazingly horrible for such a stupid reason i must make it better
        let voxels = match textures {
            TextureLoader::Client(client_texture_loader) => {

                let image = client_texture_loader.get(&save.sprite_path).get_texture_data();

                let max_voxel_y = ((image.height() * save.scale as usize) as i32 / 8) - 1;

                match &save.voxels {
                    Some(voxels) => {


                        voxels.clone()


                    },
                    None => {


                        let mut voxels: Vec<IVec2> = Vec::new();


                        for scaled_x in (0..(image.width() * save.scale as usize) as u32).step_by(8)  {
                            for scaled_y in (0..(image.height() * save.scale as usize) as u32).step_by(8)  {
                                // create an average of the 8x8 neighborhood
                                // start with bottom left

                                let mut pixel_count = 1;

                                let mut color = BLACK;
                                color.a = 0.;

                                for x_offset in 0..8 {
                                    for y_offset in 0..8 {

                                        if scaled_x + x_offset >= (image.width() * save.scale as usize) as u32 {

                                            continue;
                                        }

                                        if scaled_y + y_offset >= (image.height() * save.scale as usize) as u32 {

                                            continue;
                                        }


                                        // we need to divide by the scale to get the closest ACTUAL pixel.
                                        let pixel = image.get_pixel((scaled_x + x_offset) / save.scale as u32, (scaled_y + y_offset) / save.scale as u32);


                                        color.r += pixel.r;
                                        color.g += pixel.g;
                                        color.b += pixel.b;
                                        color.a += pixel.a;

                                        pixel_count += 1;

                                    }
                                }


                                color.r /= pixel_count as f32;
                                color.g /= pixel_count as f32;
                                color.b /= pixel_count as f32;
                                //color.a /= pixel_count as f32;


                                if color.a > 0. {

                                    let voxel_x = scaled_x as i32 / 8;
                                    let current_voxel_y = scaled_y as i32 / 8;

                                    let flipped_y = max_voxel_y - current_voxel_y;

                                    voxels.push(


                                        IVec2::new(voxel_x, flipped_y)


                                    );
                                } else {

                                }
                            }
                        }

                        voxels
                    },
                }
            },
            TextureLoader::Server(server_texture_loader) =>  {
                let image = server_texture_loader.get(&save.sprite_path);

                let max_voxel_y = ((image.height() * save.scale as u32) as i32 / 8) - 1;

                match &save.voxels {
                    Some(voxels) => voxels.clone(),
                    None => {

                        let mut voxels: Vec<IVec2> = Vec::new();

                        for scaled_x in (0..(image.width() * save.scale as u32) as u32).step_by(8)  {
                            for scaled_y in (0..(image.height() * save.scale as u32) as u32).step_by(8)  {
                                // create an average of the 8x8 neighborhood
                                // start with bottom left

                                let mut pixel_count = 1;

                                let mut color = BLACK;
                                color.a = 0.;

                                for x_offset in 0..8 {
                                    for y_offset in 0..8 {

                                        if scaled_x + x_offset >= (image.width() * save.scale as u32) as u32 {

                                            continue;
                                        }

                                        if scaled_y + y_offset >= (image.height() * save.scale as u32) as u32 {

                                            continue;
                                        }


                                        // we need to divide by the scale to get the closest ACTUAL pixel.
                                        let pixel = image.get_pixel((scaled_x + x_offset) / save.scale as u32, (scaled_y + y_offset) / save.scale as u32);


                                        color.r += pixel[0] as f32 / 255.;
                                        color.g += pixel[1] as f32 / 255.;
                                        color.b += pixel[2] as f32 / 255.;
                                        color.a += pixel[3] as f32 / 255.;

                                        pixel_count += 1;

                                    }
                                }


                                color.r /= pixel_count as f32;
                                color.g /= pixel_count as f32;
                                color.b /= pixel_count as f32;
                                //color.a /= pixel_count as f32;


                                if color.a > 0. {

                                    let voxel_x = scaled_x as i32 / 8;
                                    let current_voxel_y = scaled_y as i32 / 8;

                                    let flipped_y = max_voxel_y - current_voxel_y;


                                    voxels.push(


                                        IVec2::new(voxel_x, flipped_y)


                                    );
                                } else {

                                }
                            }
                        }

                        voxels
                    },
                }
            },
        };

        let collider_handle = space.collider_set.insert_with_parent(
            ColliderBuilder::voxels(
                glamx::Vec2::new(8., 8.,),
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
            last_received_position_update: web_time::Instant::now(),
            rigid_body_type: save.rigid_body_type,
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
            removed_voxels: save.removed_voxels,
            last_velocity_update: web_time::Instant::now(),
            spawned: web_time::Instant::now(), // this could be an issue
            lifespan: save.lifespan,
            sync_physics: save.sync_physics,
            last_ownership_change: web_time::Instant::now(), // this could also be an issue
            last_sent_position_update: web_time::Instant::now()


        }
    }

    pub fn inner_save(&self, space: &Space) -> Box<dyn PropSave> {

        let body = space.rigid_body_set.get(self.rigid_body_handle).unwrap();
        let pos = body.position().clone();

        let collider = space.collider_set.get(self.collider_handle).unwrap();
        let mass = collider.mass();


        let voxels = if self.voxels_modified {
            let coords: Vec<IVec2> = collider
                .shape()
                .as_voxels().unwrap()
                .voxels()
                .filter(|v| !v.state.is_empty())
                .map(|v| v.grid_coords)
                .collect();

            coords.into()
        } else {
            None
        };

        let save = BasePropSave {
            pos,
            mass,
            sprite_path: self.sprite_path.clone(),
            id: Some(self.id.clone()),
            owner: self.owner,
            material: self.material,
            name: self.name.clone(),
            layer: self.layer,
            voxels,
            scale: self.scale,
            rigid_body_type: self.rigid_body_type,
            removed_voxels: self.removed_voxels.clone(),
            lifespan: self.lifespan,
            sync_physics: self.sync_physics,

        };

        Box::new(save)
    }

    fn draw_mask(
        &mut self,
        ctx: &mut TickContext,
        texture: &Texture2D
    ) {

        if self.mask.is_none() {
            self.mask = Some(
                render_target(texture.width() as u32, texture.height() as u32)
            )
        }



        let mask = self.mask.as_mut().unwrap();

        ctx.add_draw_command(
            self.layer, 
            DrawCommand::SetCamera(
                SetCameraParameters {
                    rect: Rect::new(0., 0., mask.texture.width(), mask.texture.height()),
                    render_target: Some(mask.clone()),
                }
            )
        );
    


        ctx.add_draw_command(
            self.layer, 
            DrawCommand::ClearBackground(
                ClearBackgroundParameters {
                    color: WHITE,
                }
            )
        );



        for removed_voxel in &self.removed_voxels {

            // THIS MASK TEXTURE IS SCALED ALONGSIDE THE REAL TEXTURE SO THE VOXEL SIZE NEEDS TO BE DIVIDED TO KEEP IT CONSTANT
            //log::debug!("drawing masked voxel at x: {}, y: {}", removed_voxel.x * 8, removed_voxel.y * 8);
            // draw_rectangle(
            //     (removed_voxel.x as f32 * (8. / self.scale)),
            //     ((((removed_voxel.y as f32 * (8. / self.scale)) * -1.) + texture.height()) - (8. / self.scale)),
            //     8. / self.scale,
            //     8. / self.scale,
            //     BLACK
            // );

            ctx.add_draw_command(
                self.layer, 
                DrawCommand::DrawRectangle(
                    DrawRectangleParameters {
                        position: Vec2 { 
                            x: (removed_voxel.x as f32 * (8. / self.scale)), 
                            y: ((((removed_voxel.y as f32 * (8. / self.scale)) * -1.) + texture.height()) - (8. / self.scale)) 
                        },
                        size: Vec2 { 
                            x: 8. / self.scale, 
                            y: 8. / self.scale
                        },
                        offset: None,
                        rotation: None,
                        color: Some(BLACK),
                    }
                )
            );

        }

        ctx.add_draw_command(
            self.layer, 
            DrawCommand::ResetToDefaultCamera
        );
        
    }

    fn get_voxel_world_positions(
        &self,
        space: &Space
    ) -> impl Iterator<Item = (VoxelData, glamx::Vec2)> {

        let collider = space.collider_set.get(self.collider_handle).unwrap();

        let cos = collider.rotation().cos();
        let sin = collider.rotation().sin();

        collider
            .shape()
            .as_voxels()
            .unwrap()
            .voxels()
            .map(
                move |voxel|
                {
                    let rotated_x = voxel.center.x * cos - voxel.center.y * sin;
                    let rotated_y = voxel.center.x * sin + voxel.center.y * cos;

                    let world_x = rotated_x + collider.translation().x;
                    let world_y = rotated_y + collider.translation().y;

                    (voxel, glamx::Vec2::new(world_x, world_y))
                }
            )
    }

}

impl EditorContextMenu for BaseProp {
    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {
        let space = space.unwrap();

        let pos = space.rigid_body_set.get(self.rigid_body_handle).unwrap().translation();
        let size = space.collider_set.get(self.collider_handle).unwrap().shape().as_voxels().unwrap().local_aabb().half_extents();

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
            &self.inner_save(ctx.space)
        ).unwrap().into()
    }

    fn data_editor_import(&mut self, json: String, ctx: &mut crate::editor_context_menu::DataEditorContext) {
        let prop_save: BasePropSave = serde_json::from_str(&json).unwrap();

        *self = Self::from_save(prop_save, ctx.space, ctx.textures.into());
    }

    fn layer(&mut self) -> Option<&mut u32> {
        Some(&mut self.layer)
    }





}



#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq, Hash, Eq)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct NewProp {
    pub prop: Box<dyn PropSave>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePropVoxels {
    pub prop_id: PropId,
    pub area_id: AreaId,
    pub new_voxels: Vec<glamx::IVec2>,
    pub removed_voxels: Vec<glamx::IVec2> // there might be a better way to handle this
}

/// Set individual prop voxel
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetPropVoxel {
    pub prop_id: PropId,
    pub area_id: AreaId,
    pub voxel_index: glamx::IVec2,
    pub filled: bool
}
