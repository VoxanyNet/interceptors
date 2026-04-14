use std::path::PathBuf;

use glamx::{IVec2, Pose2};
use image::GenericImageView;
use macroquad::{color::BLACK};
use rapier2d::{na::base, prelude::{ColliderBuilder, RigidBodyBuilder, RigidBodyType, RigidBodyVelocity}};
use serde::{Deserialize, Serialize};

use crate::{Owner, TextureLoader, base_prop::{BaseProp, Material, PropId}, prop::Prop, prop_save::PropSave, space::Space};

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

#[typetag::serde]
impl PropSave for BasePropSave {
    fn load(&self, space: &mut Space, textures:TextureLoader) -> Box<dyn Prop>  {
        Box::new(self.inner_load(space, textures))
    }

}
impl BasePropSave {
    pub fn inner_load(&self, space: &mut Space, textures: TextureLoader) -> BaseProp {
        let body_builder = match self.rigid_body_type {
            RigidBodyType::Dynamic => RigidBodyBuilder::dynamic(),
            RigidBodyType::Fixed => RigidBodyBuilder::fixed(),
            RigidBodyType::KinematicPositionBased => RigidBodyBuilder::kinematic_position_based(),
            RigidBodyType::KinematicVelocityBased => RigidBodyBuilder::kinematic_velocity_based()
        };

        let body = space.rigid_body_set.insert(
            body_builder
                .pose(self.pos)
                //.ccd_enabled(true)
                // .soft_ccd_prediction(20.)
        );


        // this is so amazingly horrible for such a stupid reason i must make it better
        let voxels = match textures {
            TextureLoader::Client(client_texture_loader) => {

                let image = client_texture_loader.get(&self.sprite_path).get_texture_data();

                let max_voxel_y = ((image.height() * self.scale as usize) as i32 / 8) - 1;

                match &self.voxels {
                    Some(voxels) => {


                        voxels.clone()


                    },
                    None => {


                        let mut voxels: Vec<IVec2> = Vec::new(); 


                        for scaled_x in (0..(image.width() * self.scale as usize) as u32).step_by(8)  {
                            for scaled_y in (0..(image.height() * self.scale as usize) as u32).step_by(8)  {
                                // create an average of the 8x8 neighborhood
                                // start with bottom left

                                let mut pixel_count = 1;

                                let mut color = BLACK;
                                color.a = 0.;

                                for x_offset in 0..8 {
                                    for y_offset in 0..8 {

                                        if scaled_x + x_offset >= (image.width() * self.scale as usize) as u32 {

                                            continue;
                                        }

                                        if scaled_y + y_offset >= (image.height() * self.scale as usize) as u32 {

                                            continue;
                                        }


                                        // we need to divide by the scale to get the closest ACTUAL pixel.
                                        let pixel = image.get_pixel((scaled_x + x_offset) / self.scale as u32, (scaled_y + y_offset) / self.scale as u32);


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
                let image = server_texture_loader.get(&self.sprite_path);

                let max_voxel_y = ((image.height() * self.scale as u32) as i32 / 8) - 1;

                match &self.voxels {
                    Some(voxels) => voxels.clone(),
                    None => {

                        let mut voxels: Vec<IVec2> = Vec::new();

                        for scaled_x in (0..(image.width() * self.scale as u32) as u32).step_by(8)  {
                            for scaled_y in (0..(image.height() * self.scale as u32) as u32).step_by(8)  {
                                // create an average of the 8x8 neighborhood
                                // start with bottom left

                                let mut pixel_count = 1;

                                let mut color = BLACK;
                                color.a = 0.;

                                for x_offset in 0..8 {
                                    for y_offset in 0..8 {

                                        if scaled_x + x_offset >= (image.width() * self.scale as u32) as u32 {

                                            continue;
                                        }

                                        if scaled_y + y_offset >= (image.height() * self.scale as u32) as u32 {

                                            continue;
                                        }


                                        // we need to divide by the scale to get the closest ACTUAL pixel.
                                        let pixel = image.get_pixel((scaled_x + x_offset) / self.scale as u32, (scaled_y + y_offset) / self.scale as u32);


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


        let id = match self.id {
            Some(id) => id,
            None => PropId::new(),
        };




        let base_prop = BaseProp {
            last_received_position_update: web_time::Instant::now(),
            rigid_body_type: self.rigid_body_type,
            rigid_body_handle: body,
            collider_handle,
            sprite_path: self.sprite_path.clone(),
            previous_velocity: RigidBodyVelocity::zero(),
            id,
            material: self.material,
            owner: self.owner,
            last_sound_play: web_time::Instant::now(),
            despawn: false,
            last_pos_update: web_time::Instant::now(),
            name: self.name.clone(),
            context_menu_data: None,
            layer: self.layer,
            voxels_modified: self.voxels.is_some(),
            scale: self.scale,
            mask: None,
            shader_material: None,
            removed_voxels: self.removed_voxels.clone(),
            last_velocity_update: web_time::Instant::now(),
            spawned: web_time::Instant::now(), // this could be an issue
            lifespan: self.lifespan,
            sync_physics: self.sync_physics,
            last_ownership_change: web_time::Instant::now(), // this could also be an issue
            last_sent_position_update: web_time::Instant::now()


        };

        base_prop
    }
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