use std::{f64::consts::E, path::PathBuf, time::Instant};

use macroquad::math::Vec2;
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{Group, ImpulseJointHandle, InteractionGroups, RevoluteJointBuilder};
use serde::{Deserialize, Serialize};

use crate::{angle_weapon_to_mouse, body_part::BodyPart, collider_groups::{BODY_PART_GROUP, DETACHED_BODY_PART_GROUP}, player::{Facing, Player, PlayerId}, space::Space, texture_loader::TextureLoader, uuid_u64, weapon::{self, BulletImpactData, WeaponType, WeaponTypeItem, WeaponTypeSave}, ClientId, ClientTickContext};

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
    id: EnemyId,
    pub despawn: bool,
    pub weapon: Option<WeaponType>,
}

impl Enemy {

    pub fn new(position: Isometry2<f32>, owner: ClientId, space: &mut Space, weapon: Option<WeaponTypeItem>) -> Self {

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

        Self {
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
            weapon
        }
    }

    pub fn from_save(save: EnemySave, space: &mut Space) -> Self {

        let mut enemy = Self::new(save.pos, save.owner, space, None);

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
    pub fn handle_bullet_impact(&mut self, space: &mut Space, bullet_impact: BulletImpactData) {

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
    }

    pub fn despawn_if_below_level(&mut self, space: &mut Space, despawn_y: f32) {
        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation;

        if pos.y < despawn_y {
            self.despawn(space);
        }

    }

    pub fn client_tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext, players: &Vec<Player>, despawn_y: f32) {

        if self.health > 0 {
            self.upright(space, ctx);

            self.target_player(players, space);

            self.follow_target(space, players);

            
        }

        self.angle_weapon_to_mouse(space, players);
        self.head.tick(space, ctx);

        self.body.tick(space, ctx);

        self.change_facing_direction(space);

        self.detach_head_if_dead(space);

        self.despawn_if_below_level(space, despawn_y);

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
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemySave {
    pos: Isometry2<f32>,
    owner: ClientId,
    id: EnemyId,
    weapon: Option<WeaponTypeSave>
}