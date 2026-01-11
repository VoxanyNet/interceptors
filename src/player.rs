use std::{f32::consts::PI, mem::take, path::PathBuf, str::FromStr, usize};

use cs_utils::drain_filter;
use macroquad::{color::{BLACK, WHITE}, input::{KeyCode, is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_released, mouse_position, mouse_wheel}, math::{Rect, Vec2}, shapes::draw_rectangle, text::{TextParams, draw_text, draw_text_ex}, window::{screen_height, screen_width}};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{ImpulseJointHandle, RevoluteJointBuilder, RigidBody, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{ClientId, ClientTickContext, IntersectionData, Owner, Prefabs, TickContext, angle_weapon_to_mouse, area::AreaId, body_part::BodyPart, bullet_trail::BulletTrail, computer::{Item, ItemSave}, drawable::{DrawContext, Drawable}, dropped_item::{DroppedItem, RemoveDroppedItemUpdate}, enemy::Enemy, font_loader::FontLoader, get_angle_between_rapier_points, inventory::Inventory, mouse_world_pos, prop::{DissolvedPixel, Prop}, rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, tile::Tile, updates::NetworkPacket, uuid_u64, weapons::{bullet_impact_data::BulletImpactData, weapon::weapon::WeaponOwner, weapon_fire_context::WeaponFireContext, weapon_type_save::WeaponTypeSave}};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub struct PlayerId {
    id: u64
}

impl PlayerId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Facing {
    Right,
    Left
}

#[derive(Debug)]
pub struct ItemSlot {
    pub quantity: u32,
    pub item: Item
}

impl ItemSlot {
    pub fn save(&self, space: &Space) -> ItemSlotSave {
        ItemSlotSave {
            quantity: self.quantity,
            item: self.item.save(space),
        }
    }

    pub fn from_save(save: ItemSlotSave, space: &mut Space) -> ItemSlot {

        ItemSlot {
            quantity: save.quantity,
            item: Item::from_save(save.item, space),
        }
    } 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemSlotSave {
    quantity: u32,
    item: ItemSave
}

pub struct Player {
    pub id: PlayerId,
    pub health: i32,
    pub head: BodyPart,
    pub body: BodyPart,
    max_speed: Vector2<f32>,
    pub owner: Owner,
    previous_velocity: RigidBodyVelocity,
    head_joint_handle: Option<ImpulseJointHandle>,
    pub facing: Facing,
    cursor_pos_rapier: Vector2<f32>,
    previous_cursor_pos: Vector2<f32>,
    pub selected_item: usize,
    pub inventory: Inventory,
    junk: Vec<ItemSlot>, // you can hold unlimited junk
    last_changed_inventory_slot: web_time::Instant,
    pub previous_selected_item: usize,
    last_dash: web_time::Instant,
    previous_pos: Isometry2<f32>,
    last_position_update: web_time::Instant,
    last_autofire: web_time::Instant,
    flying: bool,
    pub despawn: bool,
    pub move_right_toggle: bool,
    pub move_left_toggle: bool,
}

impl Player {

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn despawn_callback(&mut self, space: &mut Space) {
        self.head.despawn(space);
        self.body.despawn(space);
        self.unequip_previous_weapon(space);
    }

    pub fn unequip_held_item(&mut self, space: &mut Space) {
        if let Some(item_slot) = &mut self.inventory.items[self.selected_item] {
            match &mut item_slot.item {
                Item::Prop(prop) => {},
                Item::Weapon(weapon_type) => {
                    weapon_type.unequip(space);
                },
            }
        }
    }
    pub fn dash(&mut self, space: &mut Space) {
        if !is_key_down(KeyCode::LeftShift) {
            return
        }

        if !(self.last_dash.elapsed().as_secs_f32() > 1.) {
            return;
        }

        let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();

        if is_key_down(KeyCode::A) {
            body.apply_impulse(vector![-1000000. * 0.4, 0.].into(), true);

            self.last_dash = web_time::Instant::now();
        }

        if is_key_down(KeyCode::D) {
            body.apply_impulse(vector![1000000. * 0.4, 0.], true);

            self.last_dash = web_time::Instant::now();
        }



    }

    pub fn equip_selected_item(&mut self, space: &mut Space) {

        if self.previous_selected_item == self.selected_item {
            return;
        }

        let item_slot = &mut self.inventory.items[self.selected_item];

        match item_slot {
            Some(item_slot) => {
                match &mut item_slot.item {
                    Item::Weapon(weapon_type) => {
                        weapon_type.equip(space, self.body.body_handle)
                    },
                    Item::Prop(_) => return
                }
            },
            None => return,
        }
    }

    pub fn pickup_item(
        &mut self, 
        dropped_items: &mut Vec<DroppedItem>, 
        space: &mut Space, 
        ctx: &mut TickContext,
        area_id: AreaId
    ) {

        let player_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector.clone();
        
        let mut picked_up_items = drain_filter(
            dropped_items, 
            |dropped_item| {
                let item_pos = space.rigid_body_set.get(dropped_item.body).unwrap().position().translation.vector;

                let distance = item_pos - player_pos;

                if distance.magnitude() < 50. {
                    
                    return true
                }

                false
                
            }
        );

        for item in &mut picked_up_items {
            item.mark_despawn();

            let packet = NetworkPacket::RemoveDroppedItemUpdate(
                RemoveDroppedItemUpdate {
                    dropped_item_id: item.id.clone(),
                    area_id,
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
        }

        'drop_item_loop: for dropped_item in picked_up_items {
            for (item_slot_index, item_slot) in &mut self.inventory.items.iter_mut().enumerate() {
                match item_slot {
                    Some(item_slot) => {

                        // matching item
                        if item_slot.item == dropped_item.item {
                            item_slot.quantity += 1;

                            let packet = NetworkPacket::ItemSlotQuantityUpdate(
                                ItemSlotQuantityUpdate {
                                    area_id: area_id,
                                    player_id: self.id,
                                    inventory_index: item_slot_index,
                                    quantity: item_slot.quantity, // we can only pick up one item at a time
                                }
                            );

                            match ctx {
                                TickContext::Client(ctx) => {
                                    ctx.network_io.send_network_packet(packet);
                                },
                                TickContext::Server(ctx) => {
                                    ctx.network_io.send_all_clients(packet);
                                },
                            };
                        }

                        continue 'drop_item_loop;
                    },
                    None => {
                        *item_slot = Some(
                            ItemSlot {
                                quantity: 1,
                                item: dropped_item.item,
                            }
                        );

                        let item_slot_save = match item_slot {
                            Some(item_slot) => Some(item_slot.save(space)),
                            None => None, // this shouldnt ever be None because we are picking up
                        };

                        let packet = NetworkPacket::ItemSlotUpdate(
                            ItemSlotUpdate {
                                area_id,
                                player_id: self.id,
                                inventory_index: item_slot_index,
                                item_slot: item_slot_save,
                            }
                        );

                        match ctx {
                            TickContext::Client(ctx) => {
                                ctx.network_io.send_network_packet(packet);
                            },
                            TickContext::Server(ctx) => {
                                ctx.network_io.send_all_clients(packet);
                            },
                        };

                        continue 'drop_item_loop;
                    },
                }
            }
        };
        


    }
    pub fn draw_hud(&self, textures: &TextureLoader) {
        
    }

    pub fn draw_inventory(&self, textures: &TextureLoader, space: &Space, prefabs: &Prefabs, fonts: &FontLoader) {

        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector;

        let mpos = rapier_to_macroquad(pos);

        for (index, item) in self.inventory.items.iter().enumerate() {

            let (slot_color, item_color) = match self.selected_item == index{
                true => {
                    let mut slot_color = BLACK;
                    let mut item_color = WHITE;
                    slot_color.a = 0.7;
                    item_color.a = 1.0;

                    (slot_color, item_color)
                },
                false => {
                    let mut slot_color = BLACK;
                    let mut item_color = WHITE;
                    slot_color.a = 0.2;
                    item_color.a = 0.7;

                    (slot_color, item_color)
                },
            };
            
            let slot_pos = Vec2 {
                x: (mpos.x + (index as f32 * 50.)) - ((50. * self.inventory.items.len() as f32) / 2. ),
                y: mpos.y - 80.
            };

            draw_rectangle(
                slot_pos.x, 
                slot_pos.y, 
                40., 
                40.,
                slot_color
            );

            // slightly offset the item to be more centered
            let item_preview_pos = Vec2 {
                x: slot_pos.x + 5.,
                y: slot_pos.y + 5.,
            };

            match item {
                Some(item_slot) => {
                    item_slot.item.draw_preview(textures, 30., item_preview_pos, prefabs, Some(item_color), 0.);
                    
                    if item_slot.quantity > 1 {
                        draw_text_ex(
                            &item_slot.quantity.to_string(), 
                            item_preview_pos.x, 
                            item_preview_pos.y + 24., 
                            TextParams {
                                font: Some(&fonts.get(PathBuf::from("assets/fonts/CutePixel.ttf"))),
                                font_size: 24,
                                color: WHITE,
                                ..Default::default()
                            }
                        );
                    }
                },
                None => {
                    
                },
            }

            
        }
    }

    pub fn set_facing(&mut self, facing: Facing, area_id: AreaId, ctx: &mut ClientTickContext) {
        self.facing = facing;

        ctx.network_io.send_network_packet(
            NetworkPacket::PlayerFacingUpdate(
                PlayerFacingUpdate { area_id: area_id, id: self.id, facing: facing }
            )
        );
    } 


    pub fn move_camera(&mut self, space: &Space, max_camera_y: f32, ctx: &mut ClientTickContext, minimum_camera_width: f32, minimum_camera_height: f32) {

        let player_position = space.rigid_body_set.get(self.body.body_handle).unwrap().translation();
        let macroquad_player_position = rapier_to_macroquad(*player_position);

        let mouse_pos: Vec2 = mouse_position().into();

        let distance_from_center = Vec2::new(
            mouse_pos.x - (ctx.camera_rect.w / 2.), 
            mouse_pos.y - (ctx.camera_rect.h / 2.)
        );

        let target_camera_pos = Vec2 {
            x: (macroquad_player_position.x - ctx.camera_rect.w / 2.) + distance_from_center.x,
            y: (macroquad_player_position.y - ctx.camera_rect.h / 2.) + distance_from_center.y,
        };

        let delta = Vec2::new(
            target_camera_pos.x - ctx.camera_rect.x,
            target_camera_pos.y - ctx.camera_rect.y
        );


        ctx.camera_rect.x += delta.x / 10.;
        ctx.camera_rect.y += delta.y / 10.;

        let ratio = screen_height() / screen_width();
        ctx.camera_rect.w = 1280.;
        ctx.camera_rect.h = ctx.camera_rect.w * ratio;    

        // camera cannot go below (above) this value
        //ctx.camera_rect.y = ctx.camera_rect.y.min(360.)
        
    }

    pub fn handle_bullet_impact(&mut self, space: &Space, bullet_impact: BulletImpactData) {
        

    }

    pub fn set_velocity(&mut self, velocity: RigidBodyVelocity , space: &mut Space) {
        space.rigid_body_set.get_mut(self.body.body_handle).unwrap().set_vels(velocity, true);
    }
    pub fn set_pos(&mut self, pos: Isometry2<f32>, space: &mut Space) {
        space.rigid_body_set.get_mut(self.body.body_handle).unwrap().set_position(pos, true);
    }

    pub fn set_cursor_pos(&mut self, pos: Vector2<f32>) {

        self.cursor_pos_rapier = pos;
    }

    pub fn new(pos: Isometry2<f32>, space: &mut Space, owner: Owner) -> Self {
        let head = BodyPart::new(PathBuf::from_str("assets/cat/head.png").unwrap(), 2, 100., pos, space, owner, Vec2::new(30., 28.));

        let body = BodyPart::new(PathBuf::from_str("assets/cat/body.png").unwrap(), 2, 1000., pos, space, owner, Vec2::new(22., 19.));

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


        let body_handle = body.body_handle.clone();

        let inventory = Inventory::new();
// OTIS DEC 2025
// pooooooooooooooooo0l0ppppppppppppppp-
        Self {
            id: PlayerId::new(),
            health: 100,
            head,
            body,
            owner,
            previous_velocity: RigidBodyVelocity::zero(),
            head_joint_handle: Some(joint),
            facing: Facing::Right,
            cursor_pos_rapier: Vector2::zeros(),
            previous_cursor_pos: Vector2::zeros(),
            max_speed: Vector2::new(350., 80.),
            selected_item: 0,
            inventory: inventory,
            last_changed_inventory_slot: web_time::Instant::now(),
            junk: Vec::new(),
            last_dash: web_time::Instant::now(),
            previous_pos: Isometry2::default(),
            last_position_update: web_time::Instant::now(),
            last_autofire: web_time::Instant::now(),
            previous_selected_item: 1,
            flying: true,
            despawn: false,
            move_left_toggle: false,
            move_right_toggle: false
        }
    }

    pub fn change_active_inventory_slot(&mut self, ctx: &mut ClientTickContext, area_id: AreaId, space: &mut Space) {

        if mouse_wheel().1 == 0. {
            return;
        }

        if mouse_wheel().1 < 0. {

            if self.selected_item == 5 {
                self.selected_item = 0;

                return;
            }

            self.selected_item += 1;

            ctx.network_io.send_network_packet(
                NetworkPacket::ActiveItemSlotUpdate(
                    ActiveItemSlotUpdate {
                        area_id,
                        player_id: self.id,
                        active_item_slot: self.selected_item as u32,
                    }
                )
            );

            
            
        } else if mouse_wheel().1 > 0. {
            if self.selected_item == 0 {
                self.selected_item = 5;
                
                return;
            }

            self.selected_item -= 1;

            ctx.network_io.send_network_packet(
                NetworkPacket::ActiveItemSlotUpdate(
                    ActiveItemSlotUpdate {
                        area_id,
                        player_id: self.id,
                        active_item_slot: self.selected_item as u32,
                    }
                )
            );

        }



    }

    pub fn use_item(
        &mut self, 
        ctx: &mut TickContext, 
        space: &mut Space, 
        props: &mut Vec<Prop>, 
        players: &mut Vec<Player>, 
        bullet_trails: &mut Vec<BulletTrail>,
        facing: Facing,
        area_id: AreaId,
        enemies: &mut Vec<Enemy>,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        dropped_items: &mut Vec<DroppedItem>,
    ) {

        if !(is_mouse_button_released(macroquad::input::MouseButton::Left) || is_mouse_button_down(macroquad::input::MouseButton::Left))  {
            return;
        }

        // take the item slot out of the inventory
        let item_slot = take(&mut self.inventory.items[self.selected_item]);

        if item_slot.is_none() {

            self.inventory.items[self.selected_item] = item_slot;

            return;

        }

        let mut item_slot = item_slot.unwrap();

        match &mut item_slot.item {
            Item::Prop(prop) => {

                //prop.use_item(&mut item_slot.quantity, ctx, space, props);
                
            },
            Item::Weapon(weapon_type) => {

                let weapon_fire_context = &mut WeaponFireContext {
                    space,
                    players,
                    props,
                    bullet_trails,
                    facing,
                    area_id,
                    dissolved_pixels,
                    enemies,
                    weapon_owner: WeaponOwner::Player(self.id),
                };
    
                weapon_type.fire(ctx, weapon_fire_context);
            }
        };

        match item_slot.quantity == 0 {
            true => {
                self.inventory.items[self.selected_item] = None;
            },
            false => self.inventory.items[self.selected_item] = Some(item_slot)
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

    pub fn control_controller(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {

        // let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();


        // if let Some(event) = ctx.gilrs.next_event() {
        //     match event.event {
        //         gilrs::EventType::ButtonPressed(button, code) => {
        //             match button {
        //                 Button::South => self.jump(body, ctx),
        //                 Button::DPadLeft => self.move_left_toggle = true,
        //                 Button::DPadRight => self.move_right_toggle = true,
        //                 _ => {}
        //             }
        //         },
        //         gilrs::EventType::ButtonReleased(button, code) => {
        //             match button {
        //                 Button::DPadLeft => self.move_left_toggle = false,
        //                 Button::DPadRight => self.move_right_toggle = false,
        //                 _ => {}
        //             }
        //         },
        //         gilrs::EventType::ButtonChanged(button, _, code) => {},
        //         gilrs::EventType::AxisChanged(axis, data, code) => {
        //             match axis {
        //                 gilrs::Axis::LeftStickX => {},
        //                 gilrs::Axis::LeftStickY => {},
        //                 gilrs::Axis::LeftZ => {},
        //                 gilrs::Axis::RightStickX => {
                           
    
        //                 },
        //                 gilrs::Axis::RightStickY => {
                            
        //                 },
        //                 gilrs::Axis::RightZ => {},
        //                 gilrs::Axis::DPadX => {},
        //                 gilrs::Axis::DPadY => {},
        //                 gilrs::Axis::Unknown => {},
        //             }
        //         },
        //         gilrs::EventType::Connected => {},
        //         gilrs::EventType::Disconnected => {},
        //         gilrs::EventType::Dropped => {},
        //         gilrs::EventType::ForceFeedbackEffectCompleted => {},
        //         _ => {},
        //     }

        // }

        // if self.move_left_toggle {
        //     self.move_left(body);
        // }

        // if self.move_right_toggle {
        //     self.move_right(body);
        // }
    }

    pub fn move_left(&mut self, body: &mut RigidBody) {
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
            Vector2::new(body.linvel().x - 50., body.linvel().y), 
            true
        );
    }

    pub fn move_right(&mut self, body: &mut RigidBody) {
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
            Vector2::new(body.linvel().x + 50., body.linvel().y), 
            true
        );
    }

    pub fn control_mkb(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();
        //self.fly(body);

        if is_key_down(KeyCode::Space) {
            self.jump(body, ctx);
        }

        if is_key_down(KeyCode::A) {
            self.move_left(body);
        }
        if is_key_down(KeyCode::D) {
            self.move_right(body);
        }
    }


    pub fn unequip_previous_weapon(&mut self, space: &mut Space) {
        if self.selected_item != self.previous_selected_item {
            match &mut self.inventory.items[self.previous_selected_item] {
                Some(item_slot) => {
                    match &mut item_slot.item {
                        Item::Prop(prop) => {},
                        Item::Weapon(weapon_type) => weapon_type.unequip(space),
                    }
                },
                None => {},
            }
        }
    }

    pub fn client_tick(
        &mut self, 
        ctx: &mut TickContext, 
        space: &mut Space, 
        area_id: AreaId,
        players: &mut Vec<Player>,
        enemies: &mut Vec<Enemy>,
        props: &mut Vec<Prop>,
        bullet_trails: &mut Vec<BulletTrail>,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        dropped_items: &mut Vec<DroppedItem>,
        max_camera_y: f32,
        minimum_camera_width: f32,
        minimum_camera_height: f32,
        tiles: &mut Vec<Vec<Option<Tile>>>

    ) {

        let current_velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().vels().clone();

        self.angle_weapon_to_mouse(space);
        self.angle_head_to_mouse(space);
        
        self.materialize_tiles(space, tiles);

        if self.owner == ctx.id() {
            self.owner_tick(
                space, 
                ctx, 
                area_id, 
                players, 
                enemies, 
                props, 
                bullet_trails, 
                dissolved_pixels, 
                dropped_items, 
                max_camera_y, 
                minimum_camera_width, 
                minimum_camera_height
            );
        }   

        self.unequip_previous_weapon(space);
        self.equip_selected_item(space);

        self.previous_selected_item = self.selected_item;
        self.previous_velocity = current_velocity;
        
    }

    pub fn face_towards_mouse(&mut self, space: &mut Space, ctx: &mut ClientTickContext, area_id: AreaId) {
        let head_body = space.rigid_body_set.get_mut(self.head.body_handle).unwrap();
        let angle_to_mouse = get_angle_between_rapier_points(head_body.position().translation.vector, self.cursor_pos_rapier);

        match angle_to_mouse.is_sign_positive() {
            true => self.set_facing(Facing::Right, area_id, ctx),
            false => self.set_facing(Facing::Left, area_id, ctx),
        }
    }
    pub fn angle_head_to_mouse(&mut self, space: &mut Space) {

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

    
    pub fn materialize_tiles(&mut self, space: &mut Space, tiles: &mut Vec<Vec<Option<Tile>>>) {

        let player_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector;

        let player_pos_tile_space = Vector2::new((player_pos.x / 50.) as usize, (player_pos.y / 50.) as usize);


        // search a 10 by 10 area for blocks to materialize
        for possible_tile_x in (player_pos_tile_space.x.saturating_sub(5))..(player_pos_tile_space.x + 5) {

            if let Some(column) = tiles.get_mut(possible_tile_x) {

                for possible_tile_y in (player_pos_tile_space.y.saturating_sub(5))..(player_pos_tile_space.y + 5) {

                    // check if this is a valid tile slot in the first place (might be out of bounds)
                    if let Some(tile_slot) = column.get_mut(possible_tile_y) {

                        // see if theres actually a tile
                        if let Some(tile) = tile_slot {
                            tile.materialize(Vector2::new(possible_tile_x, possible_tile_y), space);
                        }
                    }
                }
            }
            
        }

    }


    pub async fn draw_selected_item(&self, space: &Space, textures: &TextureLoader) {
        match &self.inventory.items[self.selected_item] {
            Some(item_slot) => {
                match &item_slot.item {
                    Item::Prop(prop) => todo!(),
                    Item::Weapon(weapon_type) => weapon_type.draw(space, textures, self.facing).await,
                }
            },
            None => {},
        }
    }

    pub fn change_facing_direction(&mut self, space: &Space, ctx: &mut ClientTickContext, area_id: AreaId) {
        let velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().linvel();


        if velocity.x > 100. {

            if !is_key_down(KeyCode::D) {
                return;
            }

            if self.facing != Facing::Right {
                self.set_facing(Facing::Right, area_id, ctx);
                
            }

        }

        if velocity.x < -100. {

            if !is_key_down(KeyCode::A) {
                return;
            }

            if self.facing != Facing::Left {
                self.set_facing(Facing::Left, area_id, ctx);

            }
        }
    }

    pub fn angle_weapon_to_mouse(&mut self, space: &mut Space) {

        match &mut self.inventory.items[self.selected_item] {
            Some(item_slot) => match &mut item_slot.item {
                Item::Prop(prop) => {return;},
                Item::Weapon(weapon_type) => {
                    angle_weapon_to_mouse(space, Some(weapon_type), self.body.body_handle, self.cursor_pos_rapier, self.facing);
                },
            },
            None => {},
        }
    }

    pub fn fly(&mut self, body: &mut RigidBody) {

        if is_key_down(KeyCode::Space) {
            body.set_linvel(
                Vector2::new(body.linvel().x, 200.), 
                true
            );
        }
    }

    pub fn jump(&mut self, body: &mut RigidBody, ctx: &mut ClientTickContext) {

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

    pub fn get_selected_item_slot_mut(&mut self) -> &mut Option<ItemSlot> {
        &mut self.inventory.items[self.selected_item]
    }

    pub fn get_selected_item_mut(&mut self) -> Option<&mut Item> {
        match &mut self.inventory.items[self.selected_item] {
            Some(item_slot) => {
                Some(&mut item_slot.item)
            },
            None => None,
        }
    }

    pub fn send_position_network_update(
        &mut self, 
        ctx: &mut TickContext, 
        space: &mut Space, 
        area_id: AreaId
    ) {

        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position();

        if (self.last_position_update.elapsed().as_secs_f32() > 3.) && *pos != self.previous_pos {

            let packet = NetworkPacket::PlayerPositionUpdate(
                PlayerPositionUpdate {
                    area_id,
                    pos: *pos,
                    player_id: self.id,
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
    
            self.last_position_update = web_time::Instant::now();
        }
    }

    pub fn send_velocity_network_update(
        &mut self, 
        ctx: &mut TickContext, 
        area_id: AreaId, 
        space: &Space
    ) {

        let current_velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().vels();

        if self.previous_velocity != *current_velocity {

            let packet = crate::updates::NetworkPacket::PlayerVelocityUpdate(
                PlayerVelocityUpdate { 
                    id: self.id.clone(), 
                    area_id, 
                    velocity: *current_velocity
                    
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
        }
    }

    pub fn owner_tick(
        &mut self, 
        space: &mut Space, 
        ctx: &mut TickContext, 
        area_id: AreaId,
        players: &mut Vec<Player>,
        enemies: &mut Vec<Enemy>,
        props: &mut Vec<Prop>,
        bullet_trails: &mut Vec<BulletTrail>,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        dropped_items: &mut Vec<DroppedItem>,
        max_camera_y: f32,
        minimum_camera_width: f32,
        minimum_camera_height: f32,
    ) {


        if let TickContext::Client(ctx) = ctx {
            self.update_cursor_pos(ctx, area_id);
            self.change_active_inventory_slot(ctx, area_id, space);
            self.change_facing_direction(space, ctx, area_id);
            self.control_controller(space, ctx);
            self.control_mkb(space, ctx);
            self.move_camera(space, max_camera_y, ctx, minimum_camera_width, minimum_camera_height);
            self.face_towards_mouse(space, ctx, area_id);
        }

        self.use_item(ctx, space, props, players, bullet_trails, self.facing, area_id, enemies, dissolved_pixels, dropped_items);
        self.send_position_network_update(ctx, space, area_id);
        self.dash(space);
        self.pickup_item(dropped_items, space, ctx, area_id);
        self.send_velocity_network_update(ctx, area_id, space);
        
        
    }


    pub fn from_save(save: PlayerSave, space: &mut Space) -> Self {
        let mut player = Self::new(save.pos, space, save.owner);

        for (index, item_slot) in save.items.iter().enumerate() {
            player.inventory.items[index] = match item_slot {
                Some(item_slot) => Some(ItemSlot::from_save(item_slot.clone(), space)),
                None => None,
            }
        }

        player.id = save.id;
        player
    }

    pub fn server_tick(&mut self) {

    }

    pub fn save(&self, space: &Space) -> PlayerSave {

        let pos = *space.rigid_body_set.get(self.body.body_handle).unwrap().position();

        let mut items = Vec::new();

        for item in &self.inventory.items {
            let item_save = match item {
                Some(item) => Some(item.save(space)),
                None => None,
            };

            items.push(item_save);
        }

        PlayerSave {
            pos,
            id: self.id.clone(),
            owner: self.owner.clone(),
            items
        }
    }
}

#[async_trait::async_trait]
impl Drawable for Player {
    async fn draw(&mut self, draw_context: &DrawContext) {

        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        self.body.draw(draw_context.textures, draw_context.space, flip_x).await;
        self.head.draw(draw_context.textures, draw_context.space, flip_x).await;

        self.draw_selected_item(draw_context.space, draw_context.textures).await;
        self.draw_inventory(draw_context.textures, draw_context.space, draw_context.prefabs, draw_context.fonts);

        let pos = draw_context.space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector;

        draw_text(&format!("{:?}", pos), draw_context.camera_rect.x + 40., draw_context.camera_rect.y + 40., 20., WHITE);
    }

    fn draw_layer(&self) -> u32 {
        1
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlayerSave {
    pos: Isometry2<f32>,
    owner: Owner,
    id: PlayerId, // we arent storing the player as a prefab so the player will always have an id
    items: Vec<Option<ItemSlotSave>>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerVelocityUpdate {
    pub id: PlayerId,
    pub area_id: AreaId,
    pub velocity: RigidBodyVelocity
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

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerFacingUpdate {
    pub area_id: AreaId,
    pub id: PlayerId,
    pub facing: Facing
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerPositionUpdate {
    pub area_id: AreaId,
    pub pos: Isometry2<f32>,
    pub player_id: PlayerId
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemSlotQuantityUpdate {
    pub area_id: AreaId,
    pub player_id: PlayerId,
    pub inventory_index: usize,
    pub quantity: u32
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveItemSlotUpdate {
    pub area_id: AreaId,
    pub player_id: PlayerId,
    pub active_item_slot: u32
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemSlotUpdate {
    pub area_id: AreaId,
    pub player_id: PlayerId,
    pub inventory_index: usize,
    pub item_slot: Option<ItemSlotSave>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveWeaponUpdate {
    pub area_id: AreaId,
    pub player_id: PlayerId,
    pub weapon: Option<WeaponTypeSave>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerHealthUpdate {
    pub area_id: AreaId,
    pub health: i32,
    pub player_id: PlayerId
}