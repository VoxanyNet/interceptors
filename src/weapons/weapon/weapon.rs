use std::path::PathBuf;

use derive_more::From;
use glamx::Pose2;
use macroquad::{audio::{PlaySoundParams, play_sound}, color::Color, input::{is_mouse_button_down, is_mouse_button_released}, math::Vec2, models::draw_mesh, rand::RandomRange};
use rapier2d::{math::Vector, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};
use serde::{Deserialize, Serialize};

use crate::{ClientId, ClientTickContext, Owner, SwapIter, TickContext, area::{self, AreaContext, AreaId}, base_prop::StupidDissolvedPixelVelocityUpdate, bullet_trail::{BulletTrail, SpawnBulletTrail}, collider_from_texture_size, draw_preview, draw_texture_onto_physics_body, drawable::{DrawContext, Drawable}, enemy::EnemyId, get_intersections, get_preview_resolution, items::{ConsumedStatus, Item, item_save::ItemSave}, player::{Facing, PlayerContext, PlayerId}, space::Space, texture_loader::ClientTextureLoader, weapons::{Weapon, ItemOwnerContext, bullet_impact_data::BulletImpactData, weapon::weapon_save::WeaponSave, weapon_fire_context::WeaponFireContext, weapon_type::ShooterContext}};


#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, From)]
pub enum WeaponOwner {
    Enemy(EnemyId),
    Player(PlayerId)
}


// common functionality that can be used as a component for a bunch of different weapon types
#[derive(PartialEq, Clone, Debug)]
pub struct BaseWeapon {
    pub player_rigid_body_handle: Option<RigidBodyHandle>,
    pub collider: Option<ColliderHandle>,
    pub rigid_body: Option<RigidBodyHandle>,
    pub sprite: PathBuf,
    pub owner: WeaponOwner,
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
    holding_fire: bool,
    facing: Facing
}

impl BaseWeapon {

    pub fn mark_despawn(&mut self) {

        self.despawn = true;
        
    }

    pub fn despawn_callback(&mut self, space: &mut Space) {
        if let Some(rigid_body) = self.rigid_body {
            space.rigid_body_set.remove(rigid_body, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
        }
    }


    pub fn new(
        owner: WeaponOwner, 
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
            holding_fire: false,
            facing

            
        }
    }

    

    pub fn handle_entity_impacts(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext,
        player_context: &mut PlayerContext,
        impacts: Vec<BulletImpactData>,
    ) {
        
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
        space: &Space,
        facing: Facing
    ) -> Vec<glamx::Vec2> {
        let mut bullet_vectors = Vec::new();

        for _ in 0..bullet_count {
            

            let bullet_vector = self.get_bullet_vector_rapier(
                space, 
                facing,
                innaccuracy
            );

            bullet_vectors.push(bullet_vector);
        };

        bullet_vectors
    }

    fn get_bullet_impacts(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext,
        weapon_owner_context: &mut ItemOwnerContext,
        bullet_vectors: Vec<glamx::Vec2>,
    ) -> Vec<BulletImpactData> {
        let mut impacts = Vec::new();
        
        for bullet_vector in bullet_vectors {

            impacts.append(&mut self.get_impacts(area_context.space, bullet_vector));
            self.create_bullet_trail(
                ctx, 
                area_context,
                weapon_owner_context,
                bullet_vector.clone(), 
            );
        };

        impacts
    }

    pub fn play_hold_fire_begin_sound(&self, ctx: &mut ClientTickContext) {

        return

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
        bullet_vectors: &Vec<glamx::Vec2>, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext
    ) {
        for bullet_vector in bullet_vectors {
            ctx.send_network_packet(
                StupidDissolvedPixelVelocityUpdate {
                    area_id: *area_context.id,
                    bullet_vector: bullet_vector.clone(),
                    weapon_pos: area_context
                        .space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap()
                        .position()
                        .translation
                }.into()
            );
        }
    }
    
    fn fire_internal(
        &mut self, 
        ctx: &mut TickContext, 
        area_context: &mut AreaContext,
        weapon_owner_context: &mut ItemOwnerContext,
        
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

        let innaccuracy_factor = 0.;
        let bullet_count = 1;
        self.reload_on_zero_bullets(ctx);

        if self.rounds == 0 {return;}

        
        self.rounds -= 1;

        if let TickContext::Client(ctx) = ctx {
            self.shake_screen(ctx);
            self.play_fire_sound(ctx);
        };
        self.last_fire = web_time::Instant::now();
        
        let facing = match weapon_owner_context {
            ItemOwnerContext::Player(player_context) => *player_context.facing,
            ItemOwnerContext::Enemy(enemy_context) => *enemy_context.facing
        };
        let bullet_vectors = self.get_bullet_vectors(
            bullet_count, 
            innaccuracy_factor, 
            area_context.space,
            facing
        );
        self.send_stupid_updates(&bullet_vectors, ctx, area_context);

        let bullet_impacts = self.get_bullet_impacts(
            ctx, 
            area_context, 
            weapon_owner_context,
            bullet_vectors
        );
        
        // optimize this
        area_context.bullet_impact_queue.extend(bullet_impacts.iter().cloned());
        
        //self.handle_entity_impacts(ctx, area_context, player_context, bullet_impacts);

    }
    
    pub fn create_bullet_trail(
        &mut self, 
        ctx: &mut TickContext, 
        area_context: &mut AreaContext,
        weapon_owner_context: &mut ItemOwnerContext,
        bullet_vector: glamx::Vec2, 
        
    ) {

        let weapon_pos = area_context.space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position();

        let owner = match weapon_owner_context {
            ItemOwnerContext::Player(player_context) => &player_context.owner,
            ItemOwnerContext::Enemy(enemy_context) => &enemy_context.owner,
        };

        let bullet_trail = BulletTrail::new(
            glamx::Vec2::new(
                weapon_pos.translation.x, 
                weapon_pos.translation.y + 10.
            ), 
            glamx::Vec2::new(
                weapon_pos.translation.x + (bullet_vector.x * 10000.),
                weapon_pos.translation.y - ((bullet_vector.y * 10000.) * -1.),
            ),
            None,
            **owner
        ); 

        let packet = crate::updates::NetworkPacket::SpawnBulletTrail(
            SpawnBulletTrail {
                area_id: *area_context.id,
                save: bullet_trail.save()
            }
        );
        ctx.send_network_packet(packet);

        area_context.bullet_trails.push(
            bullet_trail
        );

    }

    pub fn get_bullet_vector_macroquad(&mut self, _space: &Space, _facing: Facing, _innacuracy_factor: f32) {

    }

    pub fn get_bullet_vector_rapier(
        &mut self, 
        space: &Space, 
        facing: Facing,
        inaccuracy: f32
    ) -> glamx::Vec2 {
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

        let rapier_angle_bullet_vector = glamx::Vec2::new(
            macroquad_angle_bullet_vector.x,
            macroquad_angle_bullet_vector.y * -1.
        );

        rapier_angle_bullet_vector
    }
 
    pub fn get_impacts(
        &mut self, 
        space: &mut Space, 
        bullet_vector: glamx::Vec2
    ) -> Vec<BulletImpactData> {


        let pos = space.rigid_body_set.get(self.rigid_body.unwrap()).unwrap().position().translation;

        let intersections = get_intersections(
            pos, 
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
                    intersection_point: intersection.intersection_point,
                    weapon_owner: self.owner.clone(),
                    

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

impl Weapon for BaseWeapon {
    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider
    }



    fn fire(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut crate::weapons::ItemOwnerContext) {
        self.fire_internal(ctx, area_context, weapon_owner_context);
    }

    fn player_joint_handle(&self) -> Option<ImpulseJointHandle> {
        self.player_joint_handle
    }

    fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        self.rigid_body
    }
}
impl Item for BaseWeapon {


    fn use_hold(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> crate::items::ConsumedStatus {
        self.fire_internal(ctx, area_context, weapon_owner_context);

        ConsumedStatus::NotConsumed

    }

    fn use_released(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext) -> crate::items::ConsumedStatus {
        self.fire_internal(ctx, area_context, weapon_owner_context);

        ConsumedStatus::NotConsumed
    }

    fn as_weapon(&self) -> Option<&dyn crate::weapons::Weapon> {
        Some(self)
    }

    fn as_weapon_mut(&mut self) -> Option<&mut dyn Weapon> {
        Some(self)
    }

    fn same(&self, other: &dyn Item) -> bool {
        if let Some(other_concrete) = other.downcast_ref::<BaseWeapon>() {
            other_concrete == self
        } else {
            false
        }
    }
    fn stackable(&self) -> bool {
        false
    }

    fn save(&self, space: &Space) -> Box<dyn crate::items::item_save::ItemSave> {
        Box::new(
            WeaponSave {
                mass: self.mass,
                texture_size: self.texture_size,
                sprite: self.sprite.clone(),
                owner: self.owner.clone(),
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
        )
    }

    fn draw_preview(
        &self, 
        ctx: &mut TickContext,
        size: f32,
        draw_pos: Vec2,
        color: Option<Color>,
        rotation: f32
    ) {
        draw_preview(ctx, size, draw_pos, color, rotation, &self.sprite, 1);
    }

    fn get_preview_resolution(
        &self,
        textures: &ClientTextureLoader,
        size: f32
    ) -> Vec2 {
        get_preview_resolution(size, textures, &self.sprite)
    }

    fn draw_active(&self, ctx: &mut TickContext, space: &Space) {
        // dont draw if unequipped
        let rigid_body = match self.rigid_body {
            Some(rigid_body) => rigid_body,
            None => return ,
        };

        let collider = match self.collider {
            Some(collider) => collider,
            None => return ,
        };

        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        draw_texture_onto_physics_body(
            ctx,
            1,
            rigid_body, 
            collider, 
            space, 
            &self.sprite, 
            flip_x, 
            false, 
            0.,
        );
    }

    fn name(&self) -> String {
        "Unnamed weapon".to_string()
    }

    fn equip(
        &mut self, 
        ctx: &mut TickContext, 
        area_context: &mut AreaContext, 
        player_context: &mut PlayerContext
    ) {
        if self.rigid_body.is_some() {
            panic!()
        }

        let rigid_body = area_context.space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .ccd_enabled(true)
                .pose(
                    Pose2::new(
                        glamx::vec2(0., 0.),
                        0.
                    )
                )
                .build()
        ); 


        let collider = area_context.space.collider_set.insert_with_parent(
            collider_from_texture_size(self.texture_size * self.scale)
                .mass(self.mass)
                .build(), 
            rigid_body, 
            &mut area_context.space.rigid_body_set
        );
        area_context.space.collider_set.get_mut(collider).unwrap().set_collision_groups(InteractionGroups::none());

        // joint the shotgun to the player
        self.player_joint_handle = Some(area_context.space.impulse_joint_set.insert(
            player_context.body.body_handle,
            rigid_body,
            RevoluteJointBuilder::new()
                .local_anchor1(glamx::vec2(0., 0.))
                .local_anchor2(glamx::vec2(30., 0.))
                .limits([-0.8, 0.8])
                .contacts_enabled(false)
            .build(),
            true
        ));

        self.rigid_body = Some(rigid_body);
        self.collider = Some(collider);
    }

    fn unequip(
        &mut self, 
        ctx: &mut TickContext, 
        area_context: &mut AreaContext, 
        player_context: &mut PlayerContext
    ) {
        if self.rigid_body.is_none() {
            panic!()
        }

        area_context.space.rigid_body_set.remove(
            self.rigid_body.unwrap(), 
            &mut area_context.space.island_manager, 
            &mut area_context.space.collider_set, 
            &mut area_context.space.impulse_joint_set, 
            &mut area_context.space.multibody_joint_set, 
            true
        );

        self.rigid_body = None;
        self.collider = None;
        self.player_joint_handle = None;
    }

    fn tick(
        &mut self,
        ctx: &mut TickContext, 
        area_context: &mut AreaContext, 
        player_context: &mut PlayerContext
    ) {
        // this is important!
        // this is where the firing logic will come in
        todo!()
    }
}

#[async_trait::async_trait]
impl Drawable for BaseWeapon {
    async fn draw(&mut self, draw_context: &DrawContext) {

        

        
    }

    fn draw_layer(&self) -> u32 {

        
        1
    }
}


