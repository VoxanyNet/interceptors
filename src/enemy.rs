use std::{f32::consts::PI, path::PathBuf};

use glamx::{Pose2, vec2};
use macroquad::{color::{BLACK, GREEN}, math::Vec2, shapes::{draw_rectangle, draw_rectangle_lines}};
use rapier2d::{parry::query::Ray, prelude::{ColliderHandle, Group, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyVelocity}};
use serde::{Deserialize, Serialize};

use crate::{ClientTickContext, Owner, TickContext, angle_weapon_to_mouse, area::{self, AreaContext, AreaId}, base_prop::BaseProp, body_part::BodyPart, bullet_trail::BulletTrail, collider_groups::{BODY_PART_GROUP, DETACHED_BODY_PART_GROUP}, dissolved_pixel::DissolvedPixel, drawable::{DrawContext, Drawable}, get_angle_between_rapier_points, items::{Item, item_save::ItemSave}, player::{Facing, Player, PlayerId}, prop::Prop, rapier_to_macroquad, space::Space, updates::NetworkPacket, uuid_u64, weapons::{bullet_impact_data::BulletImpactData, weapon::weapon::WeaponOwner, weapon_fire_context::WeaponFireContext, weapon_type_save::WeaponTypeSave}};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct EnemyId {
    id: u64
}

impl EnemyId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Task {
    BreakingProps,
    AttackPlayer,
    ChasePlayer
}

pub struct Enemy {
    pub head: BodyPart,
    pub body: BodyPart,
    pub health: i32,
    facing: Facing,
    pub owner: Owner,
    head_body_joint: Option<ImpulseJointHandle>,
    last_jump: web_time::Instant,
    player_target: Option<PlayerId>,
    pub id: EnemyId,
    pub despawn: bool,
    pub item: Option<Box<dyn Item>>,
    pub task: Task,
    pub last_fired_weapon: web_time::Instant,
    pub last_task_change: web_time::Instant,
    pub previous_velocity: RigidBodyVelocity<f32>,
    pub previous_position: Pose2,
    pub last_position_update: web_time::Instant,
    pub last_velocity_update: web_time::Instant,
    pub last_health_update: web_time::Instant,
    pub death_time: Option<web_time::Instant>,

}

impl Enemy {

    

    pub fn angle_head_to_target(&mut self, space: &mut Space, players: &Vec<Player>) {

        let target_player = match self.player_target {
            Some(target_player_id) => players.iter().find(|player|{player.id == target_player_id}).unwrap(),
            None => return,
        };

        let target_player_pos = space.rigid_body_set.get(target_player.body.body_handle).unwrap().position().translation;


        let head_joint_handle = match self.head_body_joint {
            Some(head_joint_handle) => head_joint_handle,
            None => return,
        };

        let head_body = space.rigid_body_set.get_mut(self.head.body_handle).unwrap();

        head_body.wake_up(true);

        let angle_to_mouse = get_angle_between_rapier_points(head_body.position().translation, target_player_pos);

        let head_joint = space.impulse_joint_set.get_mut(head_joint_handle, true).unwrap();

        let target_angle = match self.facing {
            Facing::Right => {
                -angle_to_mouse + (PI / 2.)
            },
            Facing::Left => {
                (angle_to_mouse + (PI / 2.)) * -1.
            },
        };

        if target_angle.abs() > 0.399 {
            // dont try to set the angle if we know its beyond the limit
            return;
        }

        head_joint.data.as_revolute_mut().unwrap().set_motor_position(target_angle, 300., 0.);

    }

    pub fn get_colliders_between_enemy_and_target(&mut self, space: &Space, players: &Vec<Player>) -> Vec<ColliderHandle> {

        let query_pipeline = space.broad_phase.as_query_pipeline(
            space.narrow_phase.query_dispatcher(),
            &space.rigid_body_set,
            &space.collider_set,
            QueryFilter::default()
        );

        let player_target = match self.player_target {
            Some(player_target) => player_target,
            None => return Vec::new(),
        };

        let enemy_body = space.rigid_body_set.get(self.body.body_handle).unwrap();

        let player = players.iter().find(|player|{player.id == player_target}).unwrap();
        let player_body = space.rigid_body_set.get(
            player.body.body_handle
        ).unwrap();

        let line = player_body.position().translation - enemy_body.position().translation;
        
        let mut collisions = Vec::new();

        for (collider_handle, _, _) in query_pipeline.intersect_ray(
            Ray::new(enemy_body.position().translation, line), 
            line.length(), 
            true
        ) {
            if collider_handle == self.body.collider_handle || collider_handle == self.head.collider_handle {
                continue
            }

            if collider_handle == player.body.collider_handle || collider_handle == player.head.collider_handle {
                continue
            }

            if let Some(item) = &self.item {

                if let Some(weapon) = item.as_weapon() {
                    if weapon.collider_handle() == Some(collider_handle) {
                        continue;
                    }
                }
                
            }

            // if let Some(weapon) = &player.weapon {
            //     if Some(collider_handle) == weapon.collider_handle() {
            //         return true
            //     }
            // }

            let _collider_pos = space.collider_set.get(collider_handle).unwrap().position();

            collisions.push(collider_handle);

        }


        collisions
    }

    pub fn face_target(&mut self, space: &Space, players: &Vec<Player>) {

        let player_target = match self.player_target {
            Some(player_target) => player_target,
            None => return,
        };

        let target_pos = space.rigid_body_set.get(players.iter().find(|player|{player.id == player_target}).unwrap().body.body_handle).unwrap().position().translation;

        let our_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation;

        if target_pos.x < our_pos.x {
            self.facing = Facing::Left
        } else {
            self.facing = Facing::Right
        }
    }
    pub fn break_obstacles(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext 
    ) {

    
        let blocking_colliders = self.get_colliders_between_enemy_and_target(
            area_context.space, 
            area_context.players
        );
        

        let _weapon = if let Some(weapon) = &mut self.item {
            weapon
        } else {
            return;
        };

        // identify the prop that is blocking our way
        let mut blocking_prop_collider = None;

        for prop in &mut *area_context.props {
            if blocking_colliders.contains(&prop.collider_handle()) {
                blocking_prop_collider = Some(prop.collider_handle());

                break;
            }
        };
        
        if blocking_prop_collider.is_none() {
            return;
        }

        // we could maybe make it so that the enemy explicity points at the prop it wants to destroy but for now we just blindly fire the weapon if we know a prop is in the way

        self.fire_weapon(ctx, area_context);    




       
    }

    pub fn distance_to_target(&self, space: &Space, players: &Vec<Player>) -> Option<glamx::Vec2> {
        let target = match self.player_target {
            Some(player_id) => players.iter().find(|player|{player.id == player_id}).unwrap(),
            None => return None,
        };

        Some(
            space.rigid_body_set.get(target.body.body_handle)
            .unwrap()
            .position()
            .translation 
            - space.rigid_body_set.get(self.body.body_handle)
            .unwrap()
            .position()
            .translation
        )
    }


    pub fn set_task(&mut self, space: &Space, players: &Vec<Player>, props: &Vec<Box<dyn Prop>>) {
        
        if self.last_task_change.elapsed().as_secs_f32() < 0.5 {
            return;
        }

        let colliders = self.get_colliders_between_enemy_and_target(space, players);

        for prop in props {
            if colliders.contains(&prop.collider_handle()) {
                self.task = Task::BreakingProps;

                self.last_task_change = web_time::Instant::now();

                return;
            }
        }   

        if let Some(distance) = self.distance_to_target(space, players) {

            //dbg!(distance.magnitude());
            if distance.length() < 1000. {
                self.task = Task::AttackPlayer;

                return;
            }
        }

        

        // default task is to chase player
        self.task = Task::ChasePlayer;
    }


    pub fn new(
        position: glamx::Pose2, 
        owner: Owner, 
        space: &mut Space, 
        weapon: Option<Box<dyn Item>>
    ) -> Self {

        let head = BodyPart::new(
            PathBuf::from("assets/cat/head.png"), 
            2, 
            100.,
            position, 
            space, 
            owner.clone(),
            macroquad::math::Vec2::new(30., 28.)
        );

        let body = BodyPart::new(
            PathBuf::from("assets/cat/body.png"), 
            2, 
            1000.,
            position, 
            space, 
            owner.clone(),
            Vec2::new(22., 19.)
        );


        let head_body_joint = space.impulse_joint_set.insert(
            body.body_handle, 
            head.body_handle, 
            RevoluteJointBuilder::new()
                .local_anchor1(vec2(0., 0.))
                .local_anchor2(vec2(0., -30.))
                .limits([-0.4, 0.4])
                .contacts_enabled(false)
            .build(), 
            true
        );

        let enemy = Self {
            head,
            body,
            health: 30,
            facing: Facing::Right,
            owner,
            head_body_joint: Some(head_body_joint),
            last_jump: web_time::Instant::now(),
            player_target: None,
            id: EnemyId::new(),
            despawn: false,
            item: weapon,
            task: Task::ChasePlayer,
            last_fired_weapon: web_time::Instant::now(),
            last_task_change: web_time::Instant::now(),
            previous_position: Pose2::default(),
            previous_velocity: RigidBodyVelocity::zero(),
            last_position_update: web_time::Instant::now(),
            last_velocity_update: web_time::Instant::now(),
            last_health_update: web_time::Instant::now(),
            death_time: None
            
        };

        enemy
    }

    pub fn from_save(save: EnemySave, space: &mut Space) -> Self {

        let mut enemy = Self::new(save.pos, save.owner, space, None);
        
        enemy.id = save.id;


        if let Some(weapon_save) = save.weapon {

            enemy.item = Some(weapon_save.load());

        }

        enemy
    }

    pub fn save(&self, space: &Space) -> EnemySave {
        EnemySave {
            pos: *space.rigid_body_set.get(self.body.body_handle).unwrap().position(),
            owner: self.owner,
            id: self.id,
            weapon: match &self.item {
                Some(weapon) => Some(weapon.save(space)),
                None => None,
            }
        }
    } 

    #[inline]
    pub fn handle_bullet_impact(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext, 
        bullet_impact: BulletImpactData, 
    ) {

        match bullet_impact.weapon_owner {
            WeaponOwner::Enemy(_enemy_id) => return,
            WeaponOwner::Player(_player_id) => {},
        }

        // body shot
        if bullet_impact.impacted_collider == self.body.collider_handle {

            self.health -= (bullet_impact.damage * 0.5) as i32;

            area_context.space.rigid_body_set.get_mut(self.body.body_handle).unwrap().apply_impulse(bullet_impact.bullet_vector.normalize() * 100000., true);

        }
        // head shot
        else if bullet_impact.impacted_collider == self.head.collider_handle {

            let damage = bullet_impact.damage as i32;

            self.health -= damage;

            area_context.space.rigid_body_set.get_mut(self.head.body_handle).unwrap().apply_impulse(bullet_impact.bullet_vector.normalize() * 100000., true);

        }

        ctx.send_network_packet(
            NetworkPacket::EnemyHealthUpdate(
                EnemyHealthUpdate {
                    area_id: *area_context.id,
                    enemy_id: self.id,
                    health: self.health,
                }
            )
        );

        ctx.send_network_packet(
            NetworkPacket::EnemyVelocityUpdate(
                EnemyVelocityUpdate {
                    area_id: *area_context.id,
                    enemy_id: self.id,
                    velocity: *area_context.space.rigid_body_set.get(self.body.body_handle).unwrap().vels(),
                }
            )
        );

        

    }

    pub fn despawn_if_dead(&mut self, ctx: &mut TickContext, _space: &mut Space, area_id: AreaId) {
        
        if let Some(death_time) = self.death_time {
            if death_time.elapsed().as_secs_f32() > 3. {
                self.mark_despawn();

                let packet = NetworkPacket::EnemyDespawnUpdate(
                    EnemyDespawnUpdate {
                        area_id,
                        enemy_id: self.id,
                    }
                );

                ctx.send_network_packet(packet);
               
            }
        }
    }
    pub fn despawn_if_below_level(
        &mut self, 
        area_id: AreaId, 
        ctx: &mut TickContext, 
        space: &mut Space, 
        despawn_y: f32
    ) {
        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation;

        if pos.y < despawn_y {
            self.mark_despawn();

            let packet = NetworkPacket::EnemyDespawnUpdate(
                EnemyDespawnUpdate {
                    area_id,
                    enemy_id: self.id,
                }
            );
            
            ctx.send_network_packet(packet);
            
        }

    }

    pub fn fire_weapon(
        &mut self,
        ctx: &mut TickContext,
        area_context: &mut AreaContext 
    ) {

        if let Some(item) = &mut self.item {

            if let Some(weapon) = item.as_weapon_mut() {
                let enemy_context = EnemyContext {
                    head: &mut self.head,
                    body: &mut self.body,
                    health: &mut self.health,
                    facing: &mut self.facing,
                    owner: &mut self.owner,
                    head_body_joint: &mut self.head_body_joint,
                    last_jump: &mut self.last_jump,
                    player_target: &mut self.player_target,
                    id: &mut self.id,
                    despawn: &mut self.despawn,
                    weapon: &mut None, // this seems indicative of future problems
                    task: &mut self.task,
                    last_fired_weapon: &mut self.last_fired_weapon,
                    last_task_change: &mut self.last_task_change,
                    previous_velocity: &mut self.previous_velocity,
                    previous_position: &mut self.previous_position,
                    last_position_update: &mut self.last_position_update,
                    last_velocity_update: &mut self.last_velocity_update,
                    last_health_update: &mut self.last_health_update,
                    death_time: &mut self.death_time,
                };

                weapon.fire(ctx, area_context, &mut enemy_context.into());
            }
            

            self.last_fired_weapon = web_time::Instant::now();
        }


    }

    pub fn owner_tick(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext 
    ) {
        

        self.set_task(area_context.space, area_context.players, area_context.props);

        if self.health <= 0 {
            if self.death_time.is_none() {
                self.death_time = Some(web_time::Instant::now());
            }
        }


        let body = area_context.space.rigid_body_set.get(self.body.body_handle).unwrap();
        let velocity = body.vels();
        let pos = body.position();

        if *velocity != self.previous_velocity {

            let packet = NetworkPacket::EnemyVelocityUpdate(
                EnemyVelocityUpdate {
                    area_id: *area_context.id,
                    enemy_id: self.id,
                    velocity: *velocity,
                }
            );

            ctx.send_network_packet(packet);
            
        }

        if *pos != self.previous_position && self.last_position_update.elapsed().as_secs_f32() > 3. {

            let packet = NetworkPacket::EnemyPositionUpdate(
                EnemyPositionUpdate {
                    area_id: *area_context.id,
                    enemy_id: self.id,
                    position: *pos,
                }
            );

            ctx.send_network_packet(packet);
            

            self.last_position_update = web_time::Instant::now();
        }
       

        if self.health > 0 {
            match self.task {
            Task::BreakingProps => {
                self.break_obstacles(ctx, area_context);
            },
            Task::ChasePlayer => {
                self.follow_target(area_context.space, area_context.players);
            },
            Task::AttackPlayer => {

                self.fire_weapon(ctx, area_context);

                self.follow_target(area_context.space, area_context.players);

            
            }
        }
        }

        self.despawn_if_dead(ctx, area_context.space, *area_context.id);
        

        if self.despawn {
            return;
        }  



        self.despawn_if_below_level(*area_context.id, ctx, area_context.space, *area_context.despawn_y);

    }

    // pub fn send_velocity_update(&mut self) {
    //     if self.last_velocity_update.elapsed().as_secs_f32()
    // }


    pub fn set_weapon(&mut self, area_id: AreaId, ctx: &mut ClientTickContext, space: &mut Space, weapon: Box<dyn Item>) {
        self.item = Some(weapon);

        ctx.network_io.send_network_packet(
            NetworkPacket::EnemyWeaponUpdate(
                EnemyItemUpdate {
                    area_id,
                    enemy_id: self.id,
                    item: self.item.as_ref().unwrap().save(space),
                }
            )
        );
    }

    pub fn mark_despawn(&mut self) { 
        self.despawn = true;
    }


    pub fn tick(
        &mut self, 
        ctx: &mut TickContext,
        area_context: &mut AreaContext 
    ) {

        let then = web_time::Instant::now();
        
        
        if self.despawn {
            return;
        }

        self.detach_head_if_dead(area_context.space);

        

        if self.health > 0 {
            self.upright(area_context.space);

            self.target_player(area_context.players, area_context.space);

            self.angle_weapon_to_enemy(area_context.space, area_context.players);
            self.change_facing_direction(area_context.space);

            self.angle_head_to_target(area_context.space, area_context.players);

            self.face_target(area_context.space, area_context.players);

            
        }

        

        if ctx.id() == self.owner {
            self.owner_tick(ctx, area_context);
        }

        

        

    }
    
    pub fn despawn_callback(&mut self, space: &mut Space) {

        space.rigid_body_set.remove(self.body.body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
        space.rigid_body_set.remove(self.head.body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);


    }
    pub fn detach_head_if_dead(&mut self, space: &mut Space) {

        let head_joint_handle = match self.head_body_joint {
            Some(head_joint_handle) => {
                head_joint_handle
            },
            None => {
                return;
            },
        };

        if self.health <= 0 {
            space.impulse_joint_set.remove(head_joint_handle, true);

            self.head_body_joint = None;

            let new_interaction_groups = InteractionGroups::none()
                .with_memberships(DETACHED_BODY_PART_GROUP)
                .with_filter(
                    Group::ALL
                        .difference(DETACHED_BODY_PART_GROUP)
                        .difference(BODY_PART_GROUP)
                );

            
            space.collider_set.get_mut(self.head.collider_handle).unwrap().set_collision_groups(new_interaction_groups);
            space.collider_set.get_mut(self.body.collider_handle).unwrap().set_collision_groups(new_interaction_groups);
        }
    }

    pub fn follow_target(&mut self, space: &mut Space, players: &Vec<Player>) {

        let enemy_velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().linvel().clone();

        if enemy_velocity.y.abs() > 20. {
            return;
        }

        if enemy_velocity.x.abs() > 200. {
            return;
        }
        
        let target_player = match &mut self.player_target {
            Some(target_player_index) => {

                let player = players.iter().find(|player| {player.id == *target_player_index}).unwrap();

                player

            },
            None => return,
        };

        let target_player_body_translation = space.rigid_body_set.get(target_player.body.body_handle).unwrap().translation().clone();

        let enemy_body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();

        let target_vector = (target_player_body_translation - enemy_body.translation()).normalize();

        enemy_body.set_linvel(
            vec2(
                (enemy_body.linvel().x) + (10. * target_vector.x), 
                enemy_body.linvel().y
            ), 
            true
        );

    }

    pub fn target_player(&mut self, players: &Vec<Player>, space: &Space) {

        let mut lowest_distance_player: Option<PlayerId> = None;
        let mut lowest_distance: Option<f32> = None;

        let enemy_body = space.rigid_body_set.get(self.body.body_handle).unwrap();

        for player in players {
            let player_body = space.rigid_body_set.get(player.body.body_handle).unwrap();

            let distance = (player_body.translation() - enemy_body.translation()).length();

            if let Some(mut lowest_distance) = lowest_distance {
                if distance < lowest_distance {
                    lowest_distance = distance;

                    lowest_distance_player = Some(player.id);
                }
            }
            else {
                lowest_distance = Some(distance);
                lowest_distance_player = Some(player.id);

            }
        }

        // dont target players that are over 200 units away
        if let Some(lowest_distance) = lowest_distance {
            if lowest_distance > 200. {
                self.player_target = None
            }
        }

        self.player_target = lowest_distance_player;

    }

    pub fn angle_weapon_to_enemy(&mut self, space: &mut Space, players: &Vec<Player>) {

        if let Some(player_target) = self.player_target {
            let player = players.iter().find(|player|{player.id == player_target}).unwrap();

            let player_pos = space.rigid_body_set.get(player.body.body_handle).unwrap()
                .position()
                .translation;

            if let Some(item) = &mut self.item {
                if let Some(weapon) = item.as_weapon_mut() {
                    angle_weapon_to_mouse(
                        space, 
                        weapon,
                        self.body.body_handle, 
                        player_pos, 
                        self.facing
                    );
                }
            } 
            
        }
        
    }
    pub fn upright(&mut self, space: &mut Space) {
        
        let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();

        // dont try to upright if we aren't knocked over
        if body.rotation().angle().abs() < 0.5 {
            return;
        }

        // only try to jump every 3 seconds
        if self.last_jump.elapsed().as_secs_f32() > 3. {
            
            let current_velocity = body.linvel();

            // dont allow if moving if falling or jumping
            if current_velocity.y.abs() > 0.5 {
                return
            }
            
            body.set_linvel(vec2(current_velocity.x, current_velocity.y + 500.), true);

            self.last_jump = web_time::Instant::now();
        }

        let joint = space.impulse_joint_set.get_mut(self.head_body_joint.unwrap(), true).unwrap();
        joint.data.as_revolute_mut().unwrap().set_motor_position(0., 1000., 2.);
    }

    pub fn change_facing_direction(&mut self, space: &Space) {
        let velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().linvel();


        if velocity.x > 100. {

            self.facing = Facing::Right
        }

        if velocity.x < -100. {

            self.facing = Facing::Left
        }
    }

    pub fn draw_health_bar(&self, space: &Space) { 
        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation;

        let mpos = rapier_to_macroquad(pos);

        draw_rectangle_lines(mpos.x - 29., mpos.y - 64., 58., 18., 6.,BLACK);

        draw_rectangle(mpos.x - 25., mpos.y - 60., 50. * (self.health.max(0) as f32/30.), 10., GREEN);
    }
}



// #[async_trait::async_trait]
// impl Drawable for Enemy {
//     async fn draw(&mut self, draw_context: &DrawContext) {
//         if self.despawn {
//             return;
//         }

//         let flip_x = match self.facing {
//             Facing::Right => false,
//             Facing::Left => true,
//         };

//         self.body.draw(draw_context.textures, draw_context.space, flip_x).await;

//         self.head.draw(draw_context.textures, draw_context.space, flip_x).await;

//         // if let Some(weapon) = &self.item {
//         //     weapon.draw(draw_context).await
//         // }

//         self.draw_health_bar(draw_context.space);
//     }

//     fn draw_layer(&self) -> u32 {
//         3
//     }
// }

pub struct EnemyContext<'a> {
    pub head: &'a mut BodyPart,
    pub body: &'a mut BodyPart,
    pub health: &'a mut i32,
    pub facing: &'a mut Facing,
    pub owner: &'a mut Owner,
    pub head_body_joint: &'a mut Option<ImpulseJointHandle>,
    pub last_jump: &'a mut web_time::Instant,
    pub player_target: &'a mut Option<PlayerId>,
    pub id: &'a mut EnemyId,
    pub despawn: &'a mut bool,
    pub weapon: &'a mut Option<Box<dyn Item>>,
    pub task: &'a mut Task,
    pub last_fired_weapon: &'a mut web_time::Instant,
    pub last_task_change: &'a mut web_time::Instant,
    pub previous_velocity: &'a mut RigidBodyVelocity<f32>,
    pub previous_position: &'a mut Pose2,
    pub last_position_update: &'a mut web_time::Instant,
    pub last_velocity_update: &'a mut web_time::Instant,
    pub last_health_update: &'a mut web_time::Instant,
    pub death_time: &'a mut Option<web_time::Instant>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EnemySave {
    pos: Pose2,
    owner: Owner,
    id: EnemyId,
    weapon: Option<Box<dyn ItemSave>>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewEnemyUpdate {
    pub area_id: AreaId,
    pub enemy: EnemySave
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyVelocityUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub velocity: RigidBodyVelocity<f32>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyPositionUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub position: Pose2
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EnemyItemUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub item: Box<dyn ItemSave>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyDespawnUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyHealthUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub health: i32
}


