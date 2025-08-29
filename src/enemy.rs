use std::{path::PathBuf, time::Instant};

use macroquad::math::Vec2;
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{Group, ImpulseJointHandle, InteractionGroups, RevoluteJointBuilder};
use serde::{Deserialize, Serialize};

use crate::{body_part::BodyPart, collider_groups::{BODY_PART_GROUP, DETACHED_BODY_PART_GROUP}, player::{Facing, Player, PlayerId}, space::Space, texture_loader::TextureLoader, uuid_u64, weapon::BulletImpactData, ClientId, ClientTickContext};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
pub struct Enemy {
    pub head: BodyPart,
    pub body: BodyPart,
    pub health: i32,
    facing: Facing,
    pub owner: ClientId,
    head_body_joint: Option<ImpulseJointHandle>,
    last_jump: web_time::Instant,
    player_target: Option<PlayerId>,
    id: EnemyId
}

impl Enemy {

    pub fn new(position: Isometry2<f32>, owner: ClientId, space: &mut Space) -> Self {

        let head = BodyPart::new(
            PathBuf::from("assets/cat/head.png"), 
            2, 
            10.,
            position, 
            space, 
            owner.clone(),
            Vec2::new(30., 28.)
        );

        let body = BodyPart::new(
            PathBuf::from("assets/cat/body.png"), 
            2, 
            100.,
            position, 
            space, 
            owner.clone(),
            Vec2::new(22., 19.)
        );


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

        Self {
            head,
            body,
            health: 100,
            facing: Facing::Right,
            owner,
            head_body_joint: Some(head_body_joint),
            last_jump: Instant::now(),
            player_target: None,
            id: EnemyId::new()
        }
    }

    pub fn from_save(save: EnemySave, space: &mut Space) -> Self {
        Self::new(save.pos, save.owner, space)
    }

    pub fn save(&self, space: &Space) -> EnemySave {
        EnemySave {
            pos: *space.rigid_body_set.get(self.body.body_handle).unwrap().position(),
            owner: self.owner,
            id: self.id,
        }
    } 

    #[inline]
    pub fn handle_bullet_impact(&mut self, space: &mut Space, bullet_impact: BulletImpactData) {

        if self.health <= 0 {
            return;
        }

        let our_pos = space.collider_set.get(bullet_impact.impacted_collider).unwrap().position().translation;

        let distance = our_pos.vector - bullet_impact.shooter_pos.translation.vector;

        let fall_off_multiplier = (-0.01 * distance.norm()).exp();

        // body shot
        if bullet_impact.impacted_collider == self.body.collider_handle {
            let damage = (50.0 * fall_off_multiplier).round() as i32;

            self.health -= damage;

            //space.rigid_body_set.get(self.body.body_handle).unwrap().position()

        }
        // head shot
        else if bullet_impact.impacted_collider == self.head.collider_handle {

            let damage = (100.0 * fall_off_multiplier).round() as i32;

            self.health -= damage;

        }
    }

    pub fn client_tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext, players: &Vec<Player>) {

        if self.health > 0 {
            self.upright(space, ctx);

            self.target_player(players, space);

            self.follow_target(space, players);
        }
        self.head.tick(space, ctx);

        self.body.tick(space, ctx);

        self.change_facing_direction(space);

        self.detach_head_if_dead(space);

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

    pub async fn draw(&self, space: &Space, textures: &mut TextureLoader) {

        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        self.body.draw(textures, space, flip_x).await;

        self.head.draw(textures, space, flip_x).await;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemySave {
    pos: Isometry2<f32>,
    owner: ClientId,
    id: EnemyId
}