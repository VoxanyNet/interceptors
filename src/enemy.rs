use std::{f32::consts::PI, f64::consts::E, path::PathBuf, time::Instant};

use macroquad::{color::{BLACK, GREEN}, math::Vec2, rand::gen_range, shapes::{draw_rectangle, draw_rectangle_lines}};
use nalgebra::{vector, Isometry2, Vector, Vector2};
use rapier2d::{parry::query::Ray, prelude::{ColliderHandle, Group, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyVelocity}};
use serde::{Deserialize, Serialize};

use crate::{angle_weapon_to_mouse, area::AreaId, body_part::BodyPart, bullet_trail::BulletTrail, collider_groups::{BODY_PART_GROUP, DETACHED_BODY_PART_GROUP}, get_angle_between_rapier_points, player::{self, Facing, Player, PlayerId}, prop::{DissolvedPixel, Prop}, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, weapon::{self, BulletImpactData, WeaponFireContext, WeaponOwner, WeaponSave, WeaponType, WeaponTypeItem, WeaponTypeSave}, ClientId, ClientTickContext};

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

#[derive(Debug)]
pub enum Task {
    BreakingProps,
    AttackPlayer,
    ChasePlayer
}
#[derive(Debug)]
pub struct Enemy {
    pub head: BodyPart,
    pub body: BodyPart,
    pub health: i32,
    facing: Facing,
    pub owner: ClientId,
    head_body_joint: Option<ImpulseJointHandle>,
    last_jump: web_time::Instant,
    player_target: Option<PlayerId>,
    pub id: EnemyId,
    pub despawn: bool,
    pub weapon: Option<WeaponType>,
    pub task: Task,
    pub last_fired_weapon: web_time::Instant,
    pub last_task_change: web_time::Instant,
    pub previous_velocity: RigidBodyVelocity,
    pub previous_position: Isometry2<f32>,
    pub last_position_update: web_time::Instant,
    pub last_health_update: web_time::Instant
}

impl Enemy {

    pub fn angle_head_to_target(&mut self, space: &mut Space, players: &Vec<Player>) {

        let target_player = match self.player_target {
            Some(target_player_id) => players.iter().find(|player|{player.id == target_player_id}).unwrap(),
            None => return,
        };

        let target_player_pos = space.rigid_body_set.get(target_player.body.body_handle).unwrap().position().translation.vector;


        let head_joint_handle = match self.head_body_joint {
            Some(head_joint_handle) => head_joint_handle,
            None => return,
        };

        let head_body = space.rigid_body_set.get_mut(self.head.body_handle).unwrap();

        head_body.wake_up(true);

        let angle_to_mouse = get_angle_between_rapier_points(head_body.position().translation.vector, target_player_pos);

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


        let player_target = match self.player_target {
            Some(player_target) => player_target,
            None => return Vec::new(),
        };

        let enemy_body = space.rigid_body_set.get(self.body.body_handle).unwrap();

        let player = players.iter().find(|player|{player.id == player_target}).unwrap();
        let player_body = space.rigid_body_set.get(
            player.body.body_handle
        ).unwrap();

        let line = player_body.position().translation.vector - enemy_body.position().translation.vector;
        
        let mut collisions = Vec::new();
        space.query_pipeline.intersections_with_ray(
            &space.rigid_body_set, 
            &space.collider_set, 
            &Ray::new(enemy_body.position().translation.vector.into(), line.into()), 
            line.magnitude(), 
            true, 
            QueryFilter::default(),
            |collider_handle, _| {

                if collider_handle == self.body.collider_handle || collider_handle == self.head.collider_handle {
                    return true
                }

                if collider_handle == player.body.collider_handle || collider_handle == player.head.collider_handle {
                    return true
                }

                if let Some(weapon) = &self.weapon {
                    if collider_handle == weapon.collider_handle() {
                        return true;
                    }
                }

                if let Some(weapon) = &player.weapon {
                    if collider_handle == weapon.collider_handle() {
                        return true
                    }
                }

                let collider_pos = space.collider_set.get(collider_handle).unwrap().position();

                collisions.push(collider_handle);

                true
            }
        );


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
        props: &mut Vec<Prop>, 
        space: &mut Space, 
        ctx: &mut ClientTickContext, 
        players: &mut Vec<Player>, 
        bullet_trails: &mut Vec<BulletTrail>,
        area_id: AreaId,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        enemies: &mut Vec<Enemy>
    ) {

        if let Some(weapon) = &mut self.weapon {
            match weapon {
                WeaponType::Shotgun(_) => {
                    if self.last_fired_weapon.elapsed().as_secs_f32() < 1. {
                        return;
                    }
                },
            }
        } else {
            return;
        };

    
        let blocking_colliders = self.get_colliders_between_enemy_and_target(space, players);
        

        let weapon = if let Some(weapon) = &mut self.weapon {
            weapon
        } else {
            return;
        };

        // identify the prop that is blocking our way
        let mut blocking_prop_collider = None;

        for prop in &mut *props {
            if blocking_colliders.contains(&prop.collider_handle) {
                blocking_prop_collider = Some(prop.collider_handle);

                break;
            }
        };
        
        if blocking_prop_collider.is_none() {
            return;
        }

        // we could maybe make it so that the enemy explicity points at the prop it wants to destroy but for now we just blindly fire the weapon if we know a prop is in the way

        self.fire_weapon(props, space, ctx, players, bullet_trails, area_id, dissolved_pixels, enemies);    




       
    }

    pub fn distance_to_target(&self, space: &Space, players: &Vec<Player>) -> Option<Vector2<f32>> {
        let target = match self.player_target {
            Some(player_id) => players.iter().find(|player|{player.id == player_id}).unwrap(),
            None => return None,
        };

        Some(space.rigid_body_set.get(target.body.body_handle).unwrap().position().translation.vector - space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector)
    }


    pub fn set_task(&mut self, space: &Space, players: &Vec<Player>, props: &Vec<Prop>) {
        
        if self.last_task_change.elapsed().as_secs_f32() < 0.5 {
            return;
        }

        let colliders = self.get_colliders_between_enemy_and_target(space, players);

        for prop in props {
            if colliders.contains(&prop.collider_handle) {
                self.task = Task::BreakingProps;

                self.last_task_change = web_time::Instant::now();

                return;
            }
        }   

        if let Some(distance) = self.distance_to_target(space, players) {

            //dbg!(distance.magnitude());
            if distance.magnitude() < 1000. {
                self.task = Task::AttackPlayer;

                return;
            }
        }

        

        // default task is to chase player
        self.task = Task::ChasePlayer;
    }


    pub fn new(position: Isometry2<f32>, owner: ClientId, space: &mut Space, weapon: Option<WeaponTypeItem>) -> Self {

        let head = BodyPart::new(
            PathBuf::from("assets/cat/head.png"), 
            2, 
            100.,
            position, 
            space, 
            owner.clone(),
            Vec2::new(30., 28.)
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

        let weapon = match weapon {
            Some(weapon_item) => {
                Some(weapon_item.to_weapon(space, Default::default(), owner, Some(body.body_handle)))
            },
            None => None,
        };

        let head_body_joint = space.impulse_joint_set.insert(
            body.body_handle, 
            head.body_handle, 
            RevoluteJointBuilder::new()
                .local_anchor1(vector![0., 0.].into())
                .local_anchor2(vector![0., -30.].into())
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
            last_jump: Instant::now(),
            player_target: None,
            id: EnemyId::new(),
            despawn: false,
            weapon,
            task: Task::ChasePlayer,
            last_fired_weapon: web_time::Instant::now(),
            last_task_change: web_time::Instant::now(),
            previous_position: Isometry2::default(),
            previous_velocity: RigidBodyVelocity::zero(),
            last_position_update: web_time::Instant::now(),
            last_health_update: web_time::Instant::now()
            
        };

        enemy
    }

    pub fn from_save(save: EnemySave, space: &mut Space) -> Self {

        let mut enemy = Self::new(save.pos, save.owner, space, None);
        
        enemy.id = save.id;


        if let Some(weapon_save) = save.weapon {

            enemy.weapon = Some(WeaponType::from_save(space, weapon_save, Some(enemy.body.body_handle)));

        }

        enemy
    }

    pub fn save(&self, space: &Space) -> EnemySave {
        EnemySave {
            pos: *space.rigid_body_set.get(self.body.body_handle).unwrap().position(),
            owner: self.owner,
            id: self.id,
            weapon: match &self.weapon {
                Some(weapon) => Some(weapon.save(space)),
                None => None,
            }
        }
    } 

    #[inline]
    pub fn handle_bullet_impact(&mut self, area_id: AreaId, ctx: &mut ClientTickContext, space: &mut Space, bullet_impact: BulletImpactData, weapon_owner: WeaponOwner) {

        match weapon_owner {
            WeaponOwner::Enemy(enemy_id) => return,
            WeaponOwner::Player(player_id) => {},
        }

        if self.health <= 0 {
            return;
        }

        dbg!(bullet_impact.damage);
        // body shot
        if bullet_impact.impacted_collider == self.body.collider_handle {

            self.health -= (bullet_impact.damage * 0.5) as i32;

            space.rigid_body_set.get_mut(self.body.body_handle).unwrap().apply_impulse(bullet_impact.bullet_vector.normalize() * 100000., true);

        }
        // head shot
        else if bullet_impact.impacted_collider == self.head.collider_handle {

            let damage = bullet_impact.damage as i32;

            self.health -= damage;

            space.rigid_body_set.get_mut(self.head.body_handle).unwrap().apply_impulse(bullet_impact.bullet_vector.normalize() * 100000., true);

        }

        ctx.network_io.send_network_packet(
            NetworkPacket::EnemyHealthUpdate(
                EnemyHealthUpdate {
                    area_id,
                    enemy_id: self.id,
                    health: self.health,
                }
            )
        );

        ctx.network_io.send_network_packet(
            NetworkPacket::EnemyVelocityUpdate(
                EnemyVelocityUpdate {
                    area_id,
                    enemy_id: self.id,
                    velocity: *space.rigid_body_set.get(self.body.body_handle).unwrap().vels(),
                }
            )
        );

        

    }

    pub fn despawn_if_below_level(&mut self, area_id: AreaId, ctx: &mut ClientTickContext, space: &mut Space, despawn_y: f32) {
        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation;

        if pos.y < despawn_y {
            self.despawn(space);

            ctx.network_io.send_network_packet(
                NetworkPacket::EnemyDespawnUpdate(
                    EnemyDespawnUpdate {
                        area_id,
                        enemy_id: self.id,
                    }
                )
            );
            
        }

    }

    pub fn fire_weapon(&mut self, props: &mut Vec<Prop>, space: &mut Space, ctx: &mut ClientTickContext, players: &mut Vec<Player>, bullet_trails: &mut Vec<BulletTrail>, area_id: AreaId, dissolved_pixels: &mut Vec<DissolvedPixel>, enemies: &mut Vec<Enemy>) {

        if let Some(weapon) = &mut self.weapon {

            match weapon {
                WeaponType::Shotgun(shotgun) => if self.last_fired_weapon.elapsed().as_secs_f32() < 1. {
                    return;
                },
            }
            weapon.fire(ctx, &mut WeaponFireContext {
                space,
                players,
                props,
                bullet_trails,
                facing: self.facing,
                area_id,
                dissolved_pixels,
                enemies,
                weapon_owner: weapon::WeaponOwner::Enemy(self.id)
            });

            self.last_fired_weapon = web_time::Instant::now();
        }


    }

    pub fn owner_tick(
        &mut self, 
        props: &mut Vec<Prop>, 
        space: &mut Space, 
        ctx: &mut ClientTickContext, 
        players: &mut Vec<Player>, 
        bullet_trails: &mut Vec<BulletTrail>, 
        area_id: AreaId, 
        dissolved_pixels: &mut Vec<DissolvedPixel>, 
        enemies: &mut Vec<Enemy>,
        despawn_y: f32
    ) {
        
        self.set_task(space, players, props);


        let body = space.rigid_body_set.get(self.body.body_handle).unwrap();
        let velocity = body.vels();
        let pos = body.position();

        if *velocity != self.previous_velocity {
            ctx.network_io.send_network_packet(
                NetworkPacket::EnemyVelocityUpdate(
                    EnemyVelocityUpdate {
                        area_id,
                        enemy_id: self.id,
                        velocity: *velocity,
                    }
                )
            );
        }

        if *pos != self.previous_position && self.last_position_update.elapsed().as_secs_f32() < 3. {
            ctx.network_io.send_network_packet(
                NetworkPacket::EnemyPositionUpdate(
                    EnemyPositionUpdate {
                        area_id,
                        enemy_id: self.id,
                        position: *pos,
                    }
                )
            );

            self.last_position_update = web_time::Instant::now();
        }
       

        if self.health > 0 {
            match self.task {
            Task::BreakingProps => {
                self.break_obstacles(props, space, ctx, players, bullet_trails, area_id, dissolved_pixels, enemies);
            },
            Task::ChasePlayer => {
                self.follow_target(space, players);
            },
            Task::AttackPlayer => {

                self.fire_weapon(props, space, ctx, players, bullet_trails, area_id, dissolved_pixels, enemies);

                self.follow_target(space, players);

            
            }
        }
        }
        

        self.despawn_if_below_level(area_id, ctx, space, despawn_y);

    }


    pub fn set_weapon(&mut self, area_id: AreaId, ctx: &mut ClientTickContext, space: &mut Space, weapon: WeaponTypeItem) {
        self.weapon = Some(weapon.to_weapon(space, Isometry2::default(), self.owner, Some(self.body.body_handle)));

        ctx.network_io.send_network_packet(
            NetworkPacket::EnemyWeaponUpdate(
                EnemyWeaponUpdate {
                    area_id,
                    enemy_id: self.id,
                    weapon: self.weapon.as_ref().unwrap().save(space),
                }
            )
        );
    }

    pub fn client_tick(
        &mut self, 
        space: &mut Space, 
        ctx: &mut ClientTickContext, 
        players: &mut Vec<Player>, 
        despawn_y: f32,
        props: &mut Vec<Prop>,
        bullet_trails: &mut Vec<BulletTrail>,
        area_id: AreaId,
        dissolved_pixels: &mut Vec<DissolvedPixel>, enemies: &mut Vec<Enemy>,
    ) {

        if self.despawn {
            return;
        }

        self.detach_head_if_dead(space);

        

        if self.health > 0 {
            self.upright(space, ctx);

            self.target_player(players, space);

            self.angle_weapon_to_mouse(space, players);
            self.change_facing_direction(space);

            self.angle_head_to_target(space, players);

            self.face_target(space, players);

            
        }

        

        self.head.tick(space, ctx);

        self.body.tick(space, ctx);

        

        if *ctx.client_id == self.owner {
            self.owner_tick(props, space, ctx, players, bullet_trails, area_id, dissolved_pixels, enemies, despawn_y);
        }

        

        

    }
    
    pub fn despawn(&mut self, space: &mut Space) {
        self.despawn = true;

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
            vector![
                (enemy_body.linvel().x) + (10. * target_vector.x), 
                enemy_body.linvel().y
            ], 
            true
        );

    }

    pub fn target_player(&mut self, players: &Vec<Player>, space: &Space) {

        let mut lowest_distance_player: Option<PlayerId> = None;
        let mut lowest_distance: Option<f32> = None;

        let enemy_body = space.rigid_body_set.get(self.body.body_handle).unwrap();

        for player in players {
            let player_body = space.rigid_body_set.get(player.body.body_handle).unwrap();

            let distance = (player_body.translation() - enemy_body.translation()).magnitude();

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

    pub fn angle_weapon_to_mouse(&mut self, space: &mut Space, players: &Vec<Player>) {

        if let Some(player_target) = self.player_target {
            let player = players.iter().find(|player|{player.id == player_target}).unwrap();

            let player_pos = space.rigid_body_set.get(player.body.body_handle).unwrap().position().translation.vector;

            angle_weapon_to_mouse(space, self.weapon.as_mut(), self.body.body_handle, player_pos, self.facing);
        }
        
    }
    pub fn upright(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        
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
            
            body.set_linvel(vector![current_velocity.x, current_velocity.y + 500.], true);

            self.last_jump = Instant::now();
        }

        let joint = space.impulse_joint_set.get_mut(self.head_body_joint.unwrap(), true).unwrap();

        joint.data.as_revolute_mut().unwrap().set_motor_position(0., 1000., 2.);

        //println!("{:?}", joint.data.as_revolute().unwrap().motor())
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

        let mpos = rapier_to_macroquad(pos.vector);

        draw_rectangle_lines(mpos.x - 29., mpos.y - 64., 58., 18., 6.,BLACK);

        draw_rectangle(mpos.x - 25., mpos.y - 60., 50. * (self.health.max(0) as f32/30.), 10., GREEN);
    }

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {

        if self.despawn {
            return;
        }

        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        self.body.draw(textures, space, flip_x).await;

        self.head.draw(textures, space, flip_x).await;

        if let Some(weapon) = &self.weapon {
            weapon.draw(space, textures, self.facing).await
        }

        self.draw_health_bar(space);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemySave {
    pos: Isometry2<f32>,
    owner: ClientId,
    id: EnemyId,
    weapon: Option<WeaponTypeSave>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewEnemyUpdate {
    pub area_id: AreaId,
    pub enemy: EnemySave
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyVelocityUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub velocity: RigidBodyVelocity
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyPositionUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub position: Isometry2<f32>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyWeaponUpdate {
    pub area_id: AreaId,
    pub enemy_id: EnemyId,
    pub weapon: WeaponTypeSave
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


