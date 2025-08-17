use std::{collections::HashSet, time::Instant};

use macroquad::{input::{is_key_released, KeyCode}, math::Vec2};
use nalgebra::{point, vector, Vector2};
use rapier2d::{math::Translation, parry::query::Ray, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};
use serde::{Deserialize, Serialize};

use crate::{bullet_trail::BulletTrail, collider_from_texture_size, draw_texture_onto_physics_body, player::{Facing, Player}, prop::Prop, shotgun::Shotgun, space::Space, texture_loader::TextureLoader, ClientId, ClientTickContext};

pub struct WeaponFireContext<'a> {
    pub space: &'a mut Space,
    pub players: &'a mut Vec<Player>,
    pub props: &'a mut Vec<Prop>,
    pub bullet_trails: &'a mut Vec<BulletTrail>
}

#[derive(Clone)]
pub struct BulletImpactData {
    pub shooter_pos: Translation<f32>,
    pub impacted_collider: ColliderHandle
}

pub enum WeaponType {
    Shotgun(Shotgun)
}

impl WeaponType {
    pub fn fire(&mut self, ctx: &mut ClientTickContext, weapon_fire_context: &mut WeaponFireContext) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.fire(ctx, weapon_fire_context),
        }
    }

    pub fn reload(&mut self) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.reload(),
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {
        match self {
            WeaponType::Shotgun(shotgun) => shotgun.draw(space, textures).await,
        }
    }

}

pub struct Weapon {
    pub player_rigid_body_handle: Option<RigidBodyHandle>,
    pub collider: ColliderHandle,
    pub rigid_body: RigidBodyHandle,
    pub sprite: String,
    pub owner: ClientId,
    pub facing: Facing,
    pub scale: f32,
    pub aim_angle_offset: f32,
    pub fire_sound_path: String,
    pub x_screen_shake_frequency: f64,
    pub x_screen_shake_intensity: f64,
    pub y_screen_shake_frequency: f64,
    pub y_screen_shake_intensity: f64,
    pub shell_sprite: Option<String>,
    pub player_joint_handle: Option<ImpulseJointHandle>,
    last_reload: web_time::Instant,
    rounds: u32,
    capacity: u32,
    reserve_capacity: u32,
    reload_duration: web_time::Duration
}

impl Weapon {
    pub fn new(
        space: &mut Space, 
        pos: Vector2<f32>, 
        owner: ClientId, 
        player_rigid_body_handle: Option<RigidBodyHandle>,
        sprite_path: String,
        scale: f32,
        aim_angle_offset: Option<f32>,
        mass: Option<f32>,
        fire_sound_path: &str,
        x_screen_shake_frequency: f64,
        x_screen_shake_intensity: f64,
        y_screen_shake_frequency: f64,
        y_screen_shake_intensity: f64,
        shell_sprite_path: Option<String>,
        texture_size: Vec2,
        facing: Facing,
        reload_duration: web_time::Duration,
        rounds: u32,
        capacity: u32,
        reserve_capacity: u32
        

    ) -> Self {

        let mass = mass.unwrap_or(1.);

        let texture_size = texture_size * scale ; // scale the size of the shotgun
        
        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .ccd_enabled(true)
                .position(vector![pos.x, pos.y].into())
                .build()
        );

        

        let collider = space.collider_set.insert_with_parent(
            collider_from_texture_size(texture_size)
                .mass(mass)
                .build(), 
            rigid_body, 
            &mut space.rigid_body_set
        );

        let aim_angle_offset = match aim_angle_offset {
            Some(aim_angle_offset) => aim_angle_offset,
            None => 0.,
        };

        // if we are attaching the weapon to the player we need to do some epic stuff!
        let player_joint_handle: Option<ImpulseJointHandle> = if let Some(player_rigid_body_handle) = player_rigid_body_handle {

            // make the shotgun not collide with anything
            space.collider_set.get_mut(collider).unwrap().set_collision_groups(InteractionGroups::none());


            // joint the shotgun to the player
            Some(space.impulse_joint_set.insert(
                player_rigid_body_handle,
                rigid_body,
                RevoluteJointBuilder::new()
                    .local_anchor1(vector![0., 0.].into())
                    .local_anchor2(vector![30., 0.].into())
                    .limits([-0.8, 0.8])
                    .contacts_enabled(false)
                .build(),
                true
            ))
            

        } else {
            None
        };

        Self {
            player_rigid_body_handle,
            collider,
            rigid_body,
            sprite: sprite_path,
            owner: owner,
            facing,
            scale,
            aim_angle_offset,
            fire_sound_path: fire_sound_path.to_string(),
            x_screen_shake_frequency,
            x_screen_shake_intensity,
            y_screen_shake_frequency,
            y_screen_shake_intensity,
            shell_sprite: shell_sprite_path,
            player_joint_handle: player_joint_handle,
            last_reload: web_time::Instant::now(),
            rounds,
            capacity,
            reserve_capacity,
            reload_duration: reload_duration,
            
            
            
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {

        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        draw_texture_onto_physics_body(
            self.rigid_body, 
            self.collider, 
            space, 
            &self.sprite, 
            textures, 
            flip_x, 
            false, 
            0.
        ).await;

        
    }

    pub fn fire(
        &mut self, 
        ctx: &mut ClientTickContext,
        weapon_fire_context: &mut WeaponFireContext
        
    ) {

        
        // dont shoot while reloading
        if self.last_reload.elapsed() < self.reload_duration {

            // let mut sound = SoundHandle::new("assets/sounds/pistol_dry_fire.wav", [0., 0., 0.]);
            // sound.play();

            // self.sounds.push(sound);

            
            return;
        }

        
        self.rounds -= 1;

        // automatically reload if zero bullets
        if self.rounds == 0 {
            self.reload();

            // // also play dry fire sound if no bullets
            // let mut sound = SoundHandle::new("assets/sounds/pistol_dry_fire.wav", [0., 0., 0.]);
            // sound.play();

            // self.sounds.push(sound);

            return;
        }
        
        //self.shake_screen(ctx);

        //self.play_sound();

        let shotgun_body = weapon_fire_context.space.rigid_body_set.get(self.rigid_body).unwrap().clone();

        let weapon_angle = shotgun_body.rotation().angle();

        let shotgun_pos = shotgun_body.position().translation;

        let shotgun_velocity = shotgun_body.linvel();

        // we use the angle of the gun to get the direction of the bullet
        let mut macroquad_angle_bullet_vector = Vec2 {
            x:  weapon_angle.cos(),
            y: weapon_angle.sin() * -1.,
        };
        
        match self.facing {
            Facing::Right => {},
            Facing::Left => {
                macroquad_angle_bullet_vector.x *= -1.;
                macroquad_angle_bullet_vector.y *= -1.;
            }
        }

        let rapier_angle_bullet_vector = Vec2 {
            x: macroquad_angle_bullet_vector.x,
            y: macroquad_angle_bullet_vector.y * -1.
        };


        //self.knockback_player(space, rapier_angle_bullet_vector);

        weapon_fire_context.bullet_trails.push(
            BulletTrail::new(
                Vector2::new(
                    shotgun_pos.x, 
                    shotgun_pos.y + 10.
                ), 
                Vector2::new(
                    shotgun_pos.x + (macroquad_angle_bullet_vector.x * 10000.),
                    shotgun_pos.y - (macroquad_angle_bullet_vector.y * 10000.),
                ),
                None,
                self.owner.clone()
            )
        );
        
        // match &self.shell_sprite {
        //     Some(shell_sprite) => {
        //         self.bullet_casings.insert(
        //             BulletCasing::new(
        //                 Vec2::new(shotgun_pos.x, shotgun_pos.y), 
        //                 Vec2::new(5., 5.),
        //                 shell_sprite.clone(), 
        //                 space,
        //                 *shotgun_velocity
        //             )
        //         );
        //     },
        //     None => {},
        // }
        
        // from here on out needs to be cleaned up and put into seperate functions
        let ray = Ray::new(point![shotgun_pos.x, shotgun_pos.y], vector![rapier_angle_bullet_vector.x, rapier_angle_bullet_vector.y]);
        let max_toi = 5000.0;
        let solid = true;
        let filter = QueryFilter::default();

        

        let weapon_position = weapon_fire_context.space.rigid_body_set.get(self.rigid_body).unwrap().translation().clone();

        let mut hit_rigid_bodies: Vec<RigidBodyHandle> = Vec::new();
        let mut intersections: Vec<ColliderHandle> = Vec::new();
        
        // get a vector with all the intersections
        weapon_fire_context.space.query_pipeline.intersections_with_ray(&weapon_fire_context.space.rigid_body_set, &weapon_fire_context.space.collider_set, &ray, max_toi, solid, filter, 
        |handle, _intersection| {

            // dont want to intersect with the weapon itself
            if self.collider == handle {
                return true;
            };

            intersections.push(handle);

            true

        });

        // everything from here on needs to be cleaned up we are iterating way too many times
        // we probably need to invert these loops

        
        // PLAYERS
        for player in &mut *weapon_fire_context.players {

            if intersections.contains(&player.body.collider_handle) {
                let bullet_impact_data = BulletImpactData{
                    shooter_pos: shotgun_pos,
                    impacted_collider: player.body.collider_handle
                };

                player.handle_bullet_impact(weapon_fire_context.space, bullet_impact_data);

            }

            if intersections.contains(&player.head.collider_handle) {
                let bullet_impact_data = BulletImpactData{ 
                    shooter_pos: shotgun_pos, 
                    impacted_collider: player.head.collider_handle
                };

                player.handle_bullet_impact(weapon_fire_context.space, bullet_impact_data);

            }
            
        }

        for handle in intersections {
            let collider = weapon_fire_context.space.collider_set.get(handle).unwrap();
            
            hit_rigid_bodies.push(collider.parent().unwrap());

        }
        

        // apply knockback to any rigid body hit
        self.knockback_generic_rigid_bodies(&mut hit_rigid_bodies, weapon_fire_context.space, rapier_angle_bullet_vector);



    }

    // pub fn shake_screen(&self, ctx: &mut ClientTickContext) {
    //     ctx.screen_shake.x_frequency = self.x_screen_shake_frequency;
    //     ctx.screen_shake.x_intensity = self.x_screen_shake_intensity;

    //     ctx.screen_shake.x_frequency_decay = 10.;
    //     ctx.screen_shake.x_intensity_decay = 20.;
    // }

    pub fn knockback_generic_rigid_bodies(
        &self, 
        hit_rigid_bodies: &mut Vec<RigidBodyHandle>, 
        space: &mut Space,
        bullet_vector: Vec2
    ) {
        for rigid_body_handle in hit_rigid_bodies {


            let rigid_body = space.rigid_body_set.get_mut(*rigid_body_handle).unwrap();

            let mut new_velocity = rigid_body.linvel().clone();

            
            new_velocity.x += bullet_vector.x * 500.;
            new_velocity.y += bullet_vector.y * 500.;
        
            rigid_body.set_linvel(
                new_velocity, 
                true
            );
        }
    }

    pub fn reload(&mut self) {
        // dont reload while already reloading
        if self.last_reload.elapsed() < self.reload_duration {
            
            return;
        }

        // dont reload if we already have a full mag
        if self.rounds == self.capacity {
            return;
        }

        let rounds_needed_to_fill = self.capacity - self.rounds;

        // dont use rounds than are available in reserve
        let actual_rounds_available = rounds_needed_to_fill.min(self.reserve_capacity);

        if actual_rounds_available == 0 {
            // play a sound here to indicate that we cant reload
    
            return;
        }

        self.reserve_capacity -= actual_rounds_available;

        self.rounds += actual_rounds_available;

        self.last_reload = web_time::Instant::now()
    }

}
