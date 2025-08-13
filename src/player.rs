use std::f32::consts::PI;

use macroquad::{color::RED, input::{is_key_down, KeyCode}, math::{Rect, Vec2}, shapes::draw_circle};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{ColliderHandle, ImpulseJointHandle, RevoluteJointBuilder, RigidBody, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{area::{Area, AreaId}, body_part::BodyPart, get_angle_between_rapier_points, get_angle_to_mouse, mouse_world_pos, rapier_mouse_world_pos, rapier_to_macroquad, space::{self, Space}, updates::NetworkPacket, ClientId, ClientTickContext};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub struct PlayerId {
    id: u64
}

impl PlayerId {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u64_pair().0,
        }
    }
}

pub enum Facing {
    Right,
    Left
}

pub struct Player {
    pub id: PlayerId,
    health: u32,
    head: BodyPart,
    body: BodyPart,
    max_speed: Vector2<f32>,
    owner: ClientId,
    previous_pos: Isometry2<f32>,
    head_joint_handle: Option<ImpulseJointHandle>,
    facing: Facing,
    cursor_pos_rapier: Vector2<f32>,
    previous_cursor_pos: Vector2<f32>
    
}

impl Player {

    pub fn set_pos(&mut self, pos: Isometry2<f32>, space: &mut Space) {
        space.rigid_body_set.get_mut(self.body.body_handle).unwrap().set_position(pos, true);
    }

    pub fn set_cursor_pos(&mut self, pos: Vector2<f32>) {

        self.cursor_pos_rapier = pos;
    }

    pub fn new(pos: Isometry2<f32>, space: &mut Space, owner: ClientId) -> Self {
        let head = BodyPart::new("assets/cat/head.png", 2, 10., pos, space, owner, Vec2::new(30., 28.));

        let body = BodyPart::new("assets/cat/body.png", 2, 100., pos, space, owner, Vec2::new(22., 19.));

        // lock the rotation of the body
        space.rigid_body_set.get_mut(body.body_handle).unwrap().lock_rotations(true, true);

        // joint the head to the body
        let joint = space.impulse_joint_set.insert(
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
            id: PlayerId::new(),
            health: 100,
            head,
            body,
            owner,
            previous_pos: pos,
            head_joint_handle: Some(joint),
            facing: Facing::Right,
            cursor_pos_rapier: Vector2::zeros(),
            previous_cursor_pos: Vector2::zeros(),
            max_speed: Vector2::new(350., 80.)
        }
    }

    pub fn update_cursor_pos(&mut self, ctx: &mut ClientTickContext, area_id: AreaId) {
        self.cursor_pos_rapier = rapier_mouse_world_pos(ctx.camera_rect);

        if self.cursor_pos_rapier != self.previous_cursor_pos {
            ctx.network_io.send_network_packet(
                NetworkPacket::PlayerCursorUpdate(
                    PlayerCursorUpdate { area_id: area_id , id: self.id, pos: self.cursor_pos_rapier }
                )
            );
        }

        self.previous_cursor_pos = self.cursor_pos_rapier;
    }

    pub fn control(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();

        self.jump(body);

        let speed = 50.;

        if is_key_down(KeyCode::A) {
            if body.linvel().x < -self.max_speed.x {
                return;
            }

            if body.linvel().x.is_sign_positive() {
                body.set_linvel(
                    Vector2::new(body.linvel().x * 0.5, body.linvel().y), 
                    true
                );
            }

            body.set_linvel(
                Vector2::new(body.linvel().x - speed, body.linvel().y), 
                true
            );
        }

        if is_key_down(KeyCode::D) {
            if body.linvel().x > self.max_speed.x {
                return;
            }

            if body.linvel().x.is_sign_negative() {
                body.set_linvel(
                    Vector2::new(body.linvel().x * 0.5,body.linvel().y), 
                    true
                );
            }

            body.set_linvel(
                Vector2::new(body.linvel().x + speed, body.linvel().y), 
                true
            );


        }
    }

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext, space: &mut Space, area_id: AreaId) {

        let current_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().clone();
        
        self.angle_head_to_mouse(space);

        if self.owner == *ctx.client_id {
            self.owner_tick(space, ctx, area_id);
        }

        self.previous_pos = current_pos
        
    }

    pub fn angle_head_to_mouse(&mut self, space: &mut Space, ) {
        let head_joint_handle = match self.head_joint_handle {
            Some(head_joint_handle) => head_joint_handle,
            None => return,
        };

        let head_body = space.rigid_body_set.get_mut(self.head.body_handle).unwrap();

        head_body.wake_up(true);

        let angle_to_mouse = get_angle_between_rapier_points(head_body.position().translation.vector, self.cursor_pos_rapier);

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

    pub async fn draw(&self, space: &Space, textures:&mut crate::texture_loader::TextureLoader ) {
        
        self.body.draw(textures, space).await;
        self.head.draw(textures, space).await;
        
        
    }

    pub fn jump(&mut self, body: &mut RigidBody) {
        if is_key_down(KeyCode::Space) {

            // dont allow if moving, falling or jumping
            if body.linvel().y.abs() > 0.5 {
                return;
            }

            if body.linvel().y.is_sign_negative() {
                body.set_linvel(
                    Vector2::new(body.linvel().x, 0.), 
                    true
                );
            }

            body.set_linvel(
                Vector2::new(body.linvel().x, body.linvel().y + 700.), 
                true
            );
        }
    }

    pub fn owner_tick(&mut self, space: &mut Space, ctx: &mut ClientTickContext, area_id: AreaId) {
        
        self.update_cursor_pos(ctx, area_id);

        self.control(space, ctx);

        let current_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().clone();

        if self.previous_pos != current_pos {
            ctx.network_io.send_network_packet(
                crate::updates::NetworkPacket::PlayerPositionUpdate(
                    PlayerPositionUpdate { 
                        id: self.id.clone(), 
                        area_id, 
                        pos: current_pos 
                    }
                )
            );
        }
    }

    pub fn from_save(save: PlayerSave, space: &mut Space) -> Self {
        let mut player = Self::new(save.pos, space, save.owner);

        player.id = save.id;
        player
    }

    pub fn server_tick(&mut self) {

    }

    pub fn save(&self, space: &Space) -> PlayerSave {

        let pos = *space.rigid_body_set.get(self.body.body_handle).unwrap().position();

        PlayerSave {
            pos,
            id: self.id.clone(),
            owner: self.owner.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSave {
    pos: Isometry2<f32>,
    owner: ClientId,
    id: PlayerId // we arent storing the player as a prefab so the player will always have an id
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerPositionUpdate {
    pub id: PlayerId,
    pub area_id: AreaId,
    pub pos: Isometry2<f32>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewPlayer {
    pub player: PlayerSave,
    pub area_id: AreaId
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerCursorUpdate {
    pub area_id: AreaId,
    pub id: PlayerId,
    pub pos: Vector2<f32>
}