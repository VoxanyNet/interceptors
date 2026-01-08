use std::path::PathBuf;

use macroquad::{audio::{PlaySoundParams, play_sound}, color::Color, input::{is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_released}, math::Vec2, rand::RandomRange};
use nalgebra::{point, vector, Vector2};
use rapier2d::{math::Vector, parry::query::Ray, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};

use crate::{ClientId, ClientTickContext, IntersectionData, TickContext, area::AreaId, bullet_trail::{BulletTrail, SpawnBulletTrail}, collider_from_texture_size, draw_preview, draw_texture_onto_physics_body, enemy::EnemyId, get_intersections, get_preview_resolution, player::{Facing, PlayerId}, prop::StupidDissolvedPixelVelocityUpdate, space::Space, texture_loader::TextureLoader, weapons::{bullet_impact_data::BulletImpactData, weapon::weapon_save::WeaponSave, weapon_fire_context::WeaponFireContext}};


#[derive(Clone)]
pub enum WeaponOwner {
    Enemy(EnemyId),
    Player(PlayerId)
}


// common functionality that can be used as a component for a bunch of different weapon types
#[derive(PartialEq, Clone, Debug)]
pub struct WeaponBase {
    pub player_rigid_body_handle: Option<RigidBodyHandle>,
    pub collider: Option<ColliderHandle>,
    pub rigid_body: Option<RigidBodyHandle>,
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
    knockback: f32,
    texture_size: Vec2,
    mass: f32,
    pub last_fire: web_time::Instant,
    pub fire_cooldown: web_time::Duration,
    pub hold_fire_begin_sound_path: Option<PathBuf>, // worst variable name awards
    pub hold_fire_end_sound_path: Option<PathBuf>,
    holding_fire: bool
}

impl WeaponBase {

    pub fn mark_despawn(&mut self) {

        self.despawn = true;
        
    }

    pub fn despawn_callback(&mut self, space: &mut Space) {
        if let Some(rigid_body) = self.rigid_body {
            space.rigid_body_set.remove(rigid_body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
        }
    }

    pub fn equip(&mut self, space: &mut Space, player_rigid_body_handle: RigidBodyHandle) {

        if self.rigid_body.is_some() {
            panic!()
        }

        let rigid_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .ccd_enabled(true)
                .position(vector![0., 0.].into())
                .build()
        );


        let collider = space.collider_set.insert_with_parent(
            collider_from_texture_size(self.texture_size * self.scale)
                .mass(self.mass)
                .build(), 
            rigid_body, 
            &mut space.rigid_body_set
        );
        space.collider_set.get_mut(collider).unwrap().set_collision_groups(InteractionGroups::none());

        // joint the shotgun to the player
        self.player_joint_handle = Some(space.impulse_joint_set.insert(
            player_rigid_body_handle,
            rigid_body,
            RevoluteJointBuilder::new()
                .local_anchor1(vector![0., 0.].into())
                .local_anchor2(vector![30., 0.].into())
                .limits([-0.8, 0.8])
                .contacts_enabled(false)
            .build(),
            true
        ));

        self.rigid_body = Some(rigid_body);
        self.collider = Some(collider);

    }

    pub fn unequip(&mut self, space: &mut Space) {

        if self.rigid_body.is_none() {
            panic!()
        }

        space.rigid_body_set.remove(
            self.rigid_body.unwrap(), 
            &mut space.island_manager, 
            &mut space.collider_set, 
            &mut space.impulse_joint_set, 
            &mut space.multibody_joint_set, 
            true
        );

        self.rigid_body = None;
        self.collider = None;
        self.player_joint_handle = None;


        //println!("unequuop");

    }

    
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32) {
        draw_preview(textures, size, draw_pos, color, rotation, &self.sprite);
    }

    pub fn get_preview_resolution(&self, size: f32, textures: &TextureLoader) -> Vec2 {
        get_preview_resolution(size, textures, &self.sprite)
    }

    pub fn from_save(
        save: WeaponSave, 
        space: &mut Space, 
        player_rigid_body_handle: Option<RigidBodyHandle>
    ) -> Self {

        Self::new(
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
            save.knockback,
            save.fire_cooldown,
            save.hold_fire_begin_sound_path,
            save.hold_fire_end_sound_path
        )
    }

    pub fn save(&self, space: &Space) -> WeaponSave {

        let position = match self.rigid_body {
            Some(rigid_body) => {
                Some(space.rigid_body_set.get(rigid_body).unwrap().position().clone())
            },
            None => None,
        };

        WeaponSave {
            mass: self.mass,
            texture_size: self.texture_size,
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
            knockback: self.knockback,
            fire_cooldown: self.fire_cooldown,
            hold_fire_begin_sound_path: self.hold_fire_begin_sound_path.clone(),
            hold_fire_end_sound_path: self.hold_fire_end_sound_path.clone()
        }
    }
    pub fn new(
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
        knockback: f32,
        fire_cooldown: web_time::Duration,
        hold_fire_begin_sound_path: Option<PathBuf>,
        hold_fire_end_sound_path: Option<PathBuf>
        

    ) -> Self {

        let mass = mass.unwrap_or(1.);
        

        let aim_angle_offset = match aim_angle_offset {
            Some(aim_angle_offset) => aim_angle_offset,
            None => 0.,
        };

        Self {
            player_rigid_body_handle,
            collider: None,
            rigid_body: None,
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
            player_joint_handle: None,
            last_reload: web_time::Instant::now(),
            rounds,
            capacity,
            reserve_capacity,
            reload_duration: reload_duration,
            despawn: false,
            base_damage: base_damage,
            knockback,
            texture_size,
            mass,
            last_fire: web_time::Instant::now(),
            fire_cooldown,
            hold_fire_begin_sound_path,
            hold_fire_end_sound_path,
            holding_fire: false
            
        }
    }

    pub async fn draw(&self, space: &Space, textures: &TextureLoader, facing: Facing) {

        // dont draw if unequipped
        let rigid_body = match self.rigid_body {
            Some(rigid_body) => rigid_body,
            None => return ,
        };

        let collider = match self.collider {
            Some(collider) => collider,
            None => return ,
        };

        let flip_x = match facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        draw_texture_onto_physics_body(
            rigid_body, 
            collider, 
            space, 
            &self.sprite, 
            textures, 
            flip_x, 
            false, 
            0.
        ).await;

        
    }

    pub fn handle_entity_impacts(
        &mut self, 
        ctx: &mut TickContext,
        weapon_fire_context: &mut WeaponFireContext, 
        impacts: Vec<BulletImpactData>,
    ) {
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

    pub fn reload_on_zero_bullets(&mut self, ctx: &mut TickContext){
        // automatically reload if zero bullets
        if self.rounds == 0 {
            self.reload();

            if let TickContext::Client(ctx) = ctx {
                if is_mouse_button_released(macroquad::input::MouseButton::Left) {
                    let sound = ctx.sounds.get(PathBuf::from("assets\\sounds\\pistol_dry_fire.wav"));
                    play_sound(sound, PlaySoundParams {
                        looped: false,
                        volume: 0.2,
                    });
                }
            }
            

            

            

            return;
        }
    }

    pub fn play_fire_sound(&self, ctx: &mut ClientTickContext) {
        play_sound(
            ctx.sounds.get(self.fire_sound_path.clone()),
            PlaySoundParams {
                looped: false,
                volume: 0.2,
            }
        );
    }

    fn get_bullet_vectors(
        &mut self, 
        bullet_count: u32, 
        innaccuracy: f32,
        weapon_fire_context: &WeaponFireContext
    ) -> Vec<Vector2<f32>> {
        let mut bullet_vectors = Vec::new();

        for _ in 0..bullet_count {
            

            let bullet_vector = self.get_bullet_vector_rapier(
                &weapon_fire_context.space, 
                weapon_fire_context.facing,
                innaccuracy
            );

            bullet_vectors.push(bullet_vector);
        };

        bullet_vectors
    }

    fn get_bullet_impacts(
        &mut self, 
        ctx: &mut TickContext,
        bullet_vectors: Vec<Vector2<f32>>,
        weapon_fire_context: &mut WeaponFireContext
    ) -> Vec<BulletImpactData> {
        let mut impacts = Vec::new();
        
        for bullet_vector in &bullet_vectors {

            impacts.append(&mut self.get_impacts(weapon_fire_context.space, *bullet_vector));
            self.create_bullet_trail(
                ctx, 
                *bullet_vector, 
                weapon_fire_context.space, 
                weapon_fire_context.area_id, 
                weapon_fire_context.bullet_trails
            );
        };

        impacts
    }

    pub fn play_hold_fire_begin_sound(&self, ctx: &mut ClientTickContext) {

        log::debug!("Start");

        if let Some(hold_fire_begin_sound) = &self.hold_fire_begin_sound_path {
            let sound = ctx.sounds.get(hold_fire_begin_sound.clone());
            
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: 0.2,
                }
            );
        }
    }

    pub fn play_hold_fire_end_sound(&self, ctx: &mut ClientTickContext) {

        log::debug!("End");
        if let Some(hold_fire_end_sound) = &self.hold_fire_end_sound_path {
            let sound = ctx.sounds.get(hold_fire_end_sound.clone());
            
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: 0.2,
                }
            );
        }
    }

    pub fn update_holding_fire(&mut self, ctx: &mut ClientTickContext) {

        
        if is_mouse_button_down(macroquad::input::MouseButton::Left) && !self.holding_fire {

            self.holding_fire = true;

            self.play_hold_fire_begin_sound(ctx);
        }

        if !is_mouse_button_down(macroquad::input::MouseButton::Left) && self.holding_fire {
            self.holding_fire = false;

            self.play_hold_fire_end_sound(ctx);
        }
    }

    pub fn send_stupid_updates(
        &mut self, 
        bullet_vectors: &Vec<Vector2<f32>>, 
        ctx: &mut TickContext,
        weapon_fire_context: &WeaponFireContext
    ) {
        for bullet_vector in bullet_vectors {
            ctx.send_network_packet(
                StupidDissolvedPixelVelocityUpdate {
                    area_id: weapon_fire_context.area_id,
                    bullet_vector: *bullet_vector,
                    weapon_pos: weapon_fire_context.space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position().translation.vector
                }.into()
            );
        }
    }
    
    pub fn fire(
        &mut self, 
        ctx: &mut TickContext,
        weapon_fire_context: &mut WeaponFireContext,
        innaccuracy_factor: Option<f32>,
        bullet_count: Option<u32>
        
    ) {

        if let TickContext::Client(ctx) = ctx {
            self.update_holding_fire(ctx);
        };
        // dont shoot while reloading
        if self.last_reload.elapsed() < self.reload_duration {
            //let sound = ctx.sounds.get(PathBuf::from("assets\\sounds\\pistol_dry_fire.wav"));
            return;
        }

        if self.last_fire.elapsed() < self.fire_cooldown {
            return
        }

        let innaccuracy_factor = innaccuracy_factor.unwrap_or(0.);
        let bullet_count = bullet_count.unwrap_or(1);
        self.reload_on_zero_bullets(ctx);

        if self.rounds == 0 {return;}

        
        self.rounds -= 1;

        if let TickContext::Client(ctx) = ctx {
            self.shake_screen(ctx);
            self.play_fire_sound(ctx);
        };
        self.last_fire = web_time::Instant::now();
        
        let bullet_vectors = self.get_bullet_vectors(bullet_count, innaccuracy_factor, weapon_fire_context);
        self.send_stupid_updates(&bullet_vectors, ctx, weapon_fire_context);
        let bullet_impacts = self.get_bullet_impacts(ctx, bullet_vectors, weapon_fire_context);
        self.handle_entity_impacts(ctx, weapon_fire_context, bullet_impacts);

    }
    
    pub fn create_bullet_trail(&mut self, ctx: &mut TickContext, bullet_vector: Vector2<f32>, space: &Space, area_id: AreaId, bullet_trails: &mut Vec<BulletTrail>) {

        let weapon_pos = space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position();

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

        let packet = crate::updates::NetworkPacket::SpawnBulletTrail(
            SpawnBulletTrail {
                area_id: area_id,
                save: bullet_trail.save()
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

        bullet_trails.push(
            bullet_trail
        );

    }

    pub fn get_bullet_vector_macroquad(&mut self, space: &Space, facing: Facing, innacuracy_factor: f32) {

    }

    pub fn get_bullet_vector_rapier(
        &mut self, 
        space: &Space, 
        facing: Facing,
        inaccuracy: f32
    ) -> Vector2<f32> {
        let weapon_body = space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().clone();

        let mut weapon_angle = weapon_body.rotation().angle();

        let innacuracy_value = RandomRange::gen_range(-1. * inaccuracy, inaccuracy);

        weapon_angle += innacuracy_value;

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


        let pos = space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position();

        let intersections = get_intersections(
            *pos, 
            space, 
            bullet_vector, 
            self.collider
        );

        intersections.iter().map(
            |intersection| 
            {
                BulletImpactData {
                    shooter_pos: intersection.origin,
                    impacted_collider: intersection.intersected_collider,
                    bullet_vector: intersection.intersection_vector,
                    damage: self.base_damage,
                    knockback: self.knockback,
                }
            }
        ).collect()


        // space.query_pipeline.update(&space.collider_set);

        // let weapon_pos = space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position();

        // let ray = Ray::new(point![weapon_pos.translation.x, weapon_pos.translation.y], vector![bullet_vector.x, bullet_vector.y]);
        // let max_toi = 5000.0;
        // let solid = true;
        // let filter = QueryFilter::default();

        // let mut impacts = Vec::new();
        
        // space.query_pipeline.intersections_with_ray(
        //     &space.rigid_body_set, 
        //     &space.collider_set, 
        //     &ray, 
        //     max_toi, 
        //     solid, 
        //     filter, 
        //     |handle, _intersection| {

        //         if self.collider == Some(handle) {
        //             return true;
        //         };

                
        //         let pos = space.collider_set.get(handle).unwrap().position().translation;

        //         let distance = pos.vector - weapon_pos.translation.vector;

        //         let bullet_damage = self.base_damage; // (self.base_damage - distance.magnitude() * 0.1).max(0.);

        //         impacts.push(
        //             BulletImpactData {
        //                 shooter_pos: *weapon_pos,
        //                 impacted_collider: handle,
        //                 bullet_vector,
        //                 damage: bullet_damage,
        //                 knockback: self.knockback
        //             }
        //         );

        //         true

        // });

        // impacts
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


