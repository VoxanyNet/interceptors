use std::path::PathBuf;

use macroquad::{audio::{play_sound, PlaySoundParams}, color::Color, math::Vec2, rand::RandomRange};
use nalgebra::{point, vector, Vector2};
use rapier2d::{math::Vector, parry::query::Ray, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};

use crate::{area::AreaId, bullet_trail::{BulletTrail, SpawnBulletTrail}, collider_from_texture_size, draw_preview, draw_texture_onto_physics_body, enemy::EnemyId, get_preview_resolution, player::{Facing, PlayerId}, space::Space, texture_loader::TextureLoader, weapons::{bullet_impact_data::BulletImpactData, weapon::{item::WeaponItem, weapon_save::WeaponSave}, weapon_fire_context::WeaponFireContext}, ClientId, ClientTickContext};


#[derive(Clone)]
pub enum WeaponOwner {
    Enemy(EnemyId),
    Player(PlayerId)
}

#[derive(PartialEq, Clone, Debug)]
pub struct Weapon {
    pub player_rigid_body_handle: Option<RigidBodyHandle>,
    pub collider: ColliderHandle,
    pub rigid_body: RigidBodyHandle,
    pub sprite: PathBuf,
    pub owner: ClientId,
    pub scale: f32,
    pub aim_angle_offset: f32,
    pub fire_sound_path: PathBuf,
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
    reload_duration: web_time::Duration,
    despawn: bool,
    base_damage: f32,
    knockback: f32
}

impl Weapon {

    pub fn despawn(&mut self, space: &mut Space) {

        self.despawn = true;

        space.rigid_body_set.remove(self.rigid_body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

    pub fn to_item(&self, space: &Space) -> WeaponItem {

        let body = space.rigid_body_set.get(self.rigid_body).unwrap();
        let collider = space.collider_set.get(self.collider).unwrap();

        WeaponItem {
            mass: body.mass(),
            texture_size: Vec2 {
                x: collider.shape().as_cuboid().unwrap().half_extents.x,
                y: collider.shape().as_cuboid().unwrap().half_extents.y,
            },
            sprite: self.sprite.clone(),
            scale: self.scale,
            fire_sound_path: self.fire_sound_path.clone(),
            x_screen_shake_frequency: self.x_screen_shake_frequency,
            x_screen_shake_intensity: self.x_screen_shake_intensity,
            y_screen_shake_frequency: self.y_screen_shake_frequency,
            y_screen_shake_intensity: self.y_screen_shake_intensity,
            shell_sprite: self.shell_sprite.clone(),
            rounds: self.rounds,
            capacity: self.capacity,
            reserve_capacity: self.reserve_capacity,
            reload_duration: self.reload_duration.as_secs_f32(),
            base_damage: self.base_damage,
            knockback: self.knockback
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        draw_preview(textures, size, draw_pos, color, rotation, &self.sprite);
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        get_preview_resolution(size, textures, &self.sprite)
    }

    pub fn from_save(save: WeaponSave, space: &mut Space, player_rigid_body_handle: Option<RigidBodyHandle>) -> Self {

        Self::new(
            space, 
            save.pos.translation.vector, 
            save.owner, 
            player_rigid_body_handle, 
            save.sprite, 
            save.scale, 
            None, 
            Some(save.mass), 
            save.fire_sound_path, 
            save.x_screen_shake_frequency, 
            save.x_screen_shake_intensity, 
            save.y_screen_shake_frequency, 
            save.y_screen_shake_intensity, 
            save.shell_sprite, 
            save.texture_size, 
            Facing::Right, // this parameter doesnt do anything in new()
            web_time::Duration::from_secs_f32(save.reload_duration), 
            save.rounds, 
            save.capacity, 
            save.reserve_capacity,
            save.base_damage,
            save.knockback
        )
    }

    pub fn save(&self, space: &Space) -> WeaponSave {

        let body = space.rigid_body_set.get(self.rigid_body).unwrap();
        let collider = space.collider_set.get(self.collider).unwrap();

        WeaponSave {
            pos: *body.position(),
            mass: body.mass(),
            texture_size: Vec2 {
                x: collider.shape().as_cuboid().unwrap().half_extents.x,
                y: collider.shape().as_cuboid().unwrap().half_extents.y,
            },
            sprite: self.sprite.clone(),
            owner: self.owner,
            scale: self.scale,
            fire_sound_path: self.fire_sound_path.clone(),
            x_screen_shake_frequency: self.x_screen_shake_frequency,
            x_screen_shake_intensity: self.x_screen_shake_intensity,
            y_screen_shake_frequency: self.y_screen_shake_frequency,
            y_screen_shake_intensity: self.y_screen_shake_intensity,
            shell_sprite: self.shell_sprite.clone(),
            rounds: self.rounds,
            capacity: self.capacity,
            reserve_capacity: self.reserve_capacity,
            reload_duration: self.reload_duration.as_secs_f32(),
            base_damage: self.base_damage,
            knockback: self.knockback
        }
    }
    pub fn new(
        space: &mut Space, 
        pos: Vector2<f32>, 
        owner: ClientId, 
        player_rigid_body_handle: Option<RigidBodyHandle>,
        sprite_path: PathBuf,
        scale: f32,
        aim_angle_offset: Option<f32>,
        mass: Option<f32>,
        fire_sound_path: PathBuf,
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
        reserve_capacity: u32,
        base_damage: f32,
        knockback: f32
        

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
            scale,
            aim_angle_offset,
            fire_sound_path,
            x_screen_shake_frequency,
            x_screen_shake_intensity,
            y_screen_shake_frequency,
            y_screen_shake_intensity,
            shell_sprite: shell_sprite_path,
            player_joint_handle: player_joint_handle,
            last_reload: web_time::Instant::now() - web_time::Duration::from_secs(100),
            rounds,
            capacity,
            reserve_capacity,
            reload_duration: reload_duration,
            despawn: false,
            base_damage: base_damage,
            knockback
            
        }
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader, facing: Facing) {

        let flip_x = match facing {
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
        weapon_fire_context: &mut WeaponFireContext,
        innaccuracy_factor: Option<f32>,
        bullet_count: Option<u32>
        
    ) {

        dbg!(self.rounds);

        let innaccuracy_factor = innaccuracy_factor.unwrap_or(0.);
        let bullet_count = bullet_count.unwrap_or(1);
        
        // dont shoot while reloading
        if self.last_reload.elapsed() < self.reload_duration {

            let sound = ctx.sounds.get(PathBuf::from("assets\\sounds\\pistol_dry_fire.wav"));

            return;
        }


        // automatically reload if zero bullets
        if self.rounds == 0 {
            self.reload();

            let sound = ctx.sounds.get(PathBuf::from("assets\\sounds\\pistol_dry_fire.wav"));


            play_sound(sound, PlaySoundParams {
                looped: false,
                volume: 0.2,
            });

            return;
        }

        self.rounds -= 1;
        
        self.shake_screen(ctx);


        play_sound(
            ctx.sounds.get(self.fire_sound_path.clone()),
            PlaySoundParams {
                looped: false,
                volume: 0.2,
            }
        );

        

        let mut bullet_vectors = Vec::new();

        for _ in 0..bullet_count {
            

            let bullet_vector = self.get_bullet_vector_rapier(&weapon_fire_context.space, weapon_fire_context.facing);

            let innacuracy_coefficient = RandomRange::gen_range(1. - innaccuracy_factor, 1. + innaccuracy_factor);

            let innacurate_bullet_vector = Vector2::new(
                bullet_vector.x * innacuracy_coefficient, 
                bullet_vector.y * innacuracy_coefficient
            );

            bullet_vectors.push(innacurate_bullet_vector);
        };

        let mut impacts = Vec::new();
        
        for bullet_vector in &bullet_vectors {

        
            impacts.append(&mut self.get_impacts(weapon_fire_context.space, *bullet_vector));

            self.create_bullet_trail(ctx, *bullet_vector, weapon_fire_context.space, weapon_fire_context.area_id, weapon_fire_context.bullet_trails);

        };
        
        // PLAYERS
        for player in &mut *weapon_fire_context.players {

            let body_collider = player.body.collider_handle;
            let head_collider = player.head.collider_handle;

            for impact in  impacts.iter().filter(|intersection| {intersection.impacted_collider == body_collider || intersection.impacted_collider == head_collider}) {
                player.handle_bullet_impact(&weapon_fire_context.space, impact.clone());
            };
            
            
        }

        for enemy in &mut *weapon_fire_context.enemies {

            let body_collider = enemy.body.collider_handle;
            let head_collider = enemy.head.collider_handle;

            for impact in  impacts.iter().filter(|intersection| {intersection.impacted_collider == body_collider || intersection.impacted_collider == head_collider}) {
                enemy.handle_bullet_impact(weapon_fire_context.area_id, ctx, &mut weapon_fire_context.space, impact.clone(), weapon_fire_context.weapon_owner.clone());

                break;
            };
        }

        for prop in &mut *weapon_fire_context.props {

            let collider = prop.collider_handle;

            for impact in impacts.iter().filter(|impact| {impact.impacted_collider == collider}) {
                prop.handle_bullet_impact(ctx, &impact, weapon_fire_context.space, weapon_fire_context.area_id, weapon_fire_context.dissolved_pixels);
            };

            

            
        }

        for bullet_vector in &bullet_vectors {
        
            impacts.append(&mut self.get_impacts(weapon_fire_context.space, *bullet_vector));

        };

        for dissolved_pixel in &mut *weapon_fire_context.dissolved_pixels {

            let collider = dissolved_pixel.collider;

            for impact in impacts.iter().filter(|impact| {impact.impacted_collider == collider}) {
                let body = weapon_fire_context.space.rigid_body_set.get_mut(dissolved_pixel.body).unwrap();
                body.apply_impulse(
                    Vector::new(impact.bullet_vector.x * 5000., impact.bullet_vector.y * 5000.), 
                    true
                );
            }
        };
            

    }
    
    pub fn create_bullet_trail(&mut self, ctx: &mut ClientTickContext, bullet_vector: Vector2<f32>, space: &Space, area_id: AreaId, bullet_trails: &mut Vec<BulletTrail>) {

        let weapon_pos = space.rigid_body_set.get(self.rigid_body).unwrap().position();

        let bullet_trail = BulletTrail::new(
            Vector2::new(
                weapon_pos.translation.x, 
                weapon_pos.translation.y + 10.
            ), 
            Vector2::new(
                weapon_pos.translation.x + (bullet_vector.x * 10000.),
                weapon_pos.translation.y - ((bullet_vector.y * 10000.) * -1.),
            ),
            None,
            self.owner.clone()
        ); 


        ctx.network_io.send_network_packet(
            crate::updates::NetworkPacket::SpawnBulletTrail(SpawnBulletTrail {
                area_id: area_id,
                save: bullet_trail.save()
            })
        );

        bullet_trails.push(
            bullet_trail
        );
    }

    pub fn get_bullet_vector_macroquad(&mut self, space: &Space, facing: Facing, innacuracy_factor: f32) {

    }

    pub fn get_bullet_vector_rapier(&mut self, space: &Space, facing: Facing) -> Vector2<f32> {
        let weapon_body = space.rigid_body_set.get(self.rigid_body).unwrap().clone();

        let weapon_angle = weapon_body.rotation().angle();

        // we use the angle of the gun to get the direction of the bullet
        let mut macroquad_angle_bullet_vector = Vec2 {
            x:  weapon_angle.cos(),
            y: weapon_angle.sin() * -1.,
        };
        
        match facing {
            Facing::Right => {},
            Facing::Left => {
                macroquad_angle_bullet_vector.x *= -1.;
                macroquad_angle_bullet_vector.y *= -1.;
            }
        }

        let rapier_angle_bullet_vector = Vector2::new(
            macroquad_angle_bullet_vector.x,
            macroquad_angle_bullet_vector.y * -1.
        );

        rapier_angle_bullet_vector
    }
    
    pub fn get_impacts(&mut self, space: &mut Space, bullet_vector: Vector2<f32>) -> Vec<BulletImpactData> {

        space.query_pipeline.update(&space.collider_set);

        let weapon_pos = space.rigid_body_set.get(self.rigid_body).unwrap().position();

        let ray = Ray::new(point![weapon_pos.translation.x, weapon_pos.translation.y], vector![bullet_vector.x, bullet_vector.y]);
        let max_toi = 5000.0;
        let solid = true;
        let filter = QueryFilter::default();

        let mut impacts = Vec::new();
        
        space.query_pipeline.intersections_with_ray(
            &space.rigid_body_set, 
            &space.collider_set, 
            &ray, 
            max_toi, 
            solid, 
            filter, 
            |handle, _intersection| {

                if self.collider == handle {
                    return true;
                };

                
                let pos = space.collider_set.get(handle).unwrap().position().translation;

                let distance = pos.vector - weapon_pos.translation.vector;

                let bullet_damage = self.base_damage; // (self.base_damage - distance.magnitude() * 0.1).max(0.);

                impacts.push(
                    BulletImpactData {
                        shooter_pos: *weapon_pos,
                        impacted_collider: handle,
                        bullet_vector,
                        damage: bullet_damage,
                        knockback: self.knockback
                    }
                );

                true

        });

        impacts
    }

    pub fn shake_screen(&self, ctx: &mut ClientTickContext) {
        ctx.screen_shake.x_frequency = self.x_screen_shake_frequency;
        ctx.screen_shake.x_intensity = self.x_screen_shake_intensity;

        ctx.screen_shake.x_frequency_decay = 10.;
        ctx.screen_shake.x_intensity_decay = 20.;
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


