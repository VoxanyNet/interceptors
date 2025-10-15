use std::{f32::consts::PI, mem::take, path::PathBuf, str::FromStr, time::Instant, usize};

use cs_utils::drain_filter;
use macroquad::{color::{BLACK, RED, WHITE}, input::{is_key_down, is_mouse_button_down, is_mouse_button_released, mouse_wheel, KeyCode}, math::{Rect, Vec2}, shapes::{draw_circle, draw_rectangle}, text::{draw_text, draw_text_ex, TextParams}, window::{screen_height, screen_width}};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::{ImpulseJointHandle, RevoluteJointBuilder, RigidBody, RigidBodyVelocity};
use serde::{Deserialize, Serialize};

use crate::{angle_weapon_to_mouse, area::AreaId, body_part::BodyPart, bullet_trail::BulletTrail, computer::{Item, ItemSave}, dropped_item::{DroppedItem, RemoveDroppedItemUpdate}, enemy::Enemy, font_loader::FontLoader, get_angle_between_rapier_points, inventory::Inventory, prop::{DissolvedPixel, Prop}, rapier_mouse_world_pos, rapier_to_macroquad, round_to_nearest, space::Space, texture_loader::TextureLoader, tile::Tile, updates::NetworkPacket, uuid_u64, weapons::{bullet_impact_data::BulletImpactData, weapon::weapon::WeaponOwner, weapon_fire_context::WeaponFireContext, weapon_type::WeaponType, weapon_type_save::WeaponTypeSave}, ClientId, ClientTickContext, Prefabs};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub owner: ClientId,
    previous_velocity: RigidBodyVelocity,
    head_joint_handle: Option<ImpulseJointHandle>,
    facing: Facing,
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
    flying: bool
}

impl Player {

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

    pub fn equip_selected_weapon(&mut self, space: &mut Space) {

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
        ctx: &mut ClientTickContext,
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
            item.despawn(space);

            ctx.network_io.send_network_packet(
                NetworkPacket::RemoveDroppedItemUpdate(
                    RemoveDroppedItemUpdate {
                        dropped_item_id: item.id.clone(),
                        area_id,
                    }
                )
            );
        }

        'drop_item_loop: for dropped_item in picked_up_items {
            for (item_slot_index, item_slot) in &mut self.inventory.items.iter_mut().enumerate() {
                match item_slot {
                    Some(item_slot) => {

                        // matching item
                        if item_slot.item == dropped_item.item {
                            item_slot.quantity += 1;

                            ctx.network_io.send_network_packet(
                                NetworkPacket::ItemSlotQuantityUpdate(
                                    ItemSlotQuantityUpdate {
                                        area_id: area_id,
                                        player_id: self.id,
                                        inventory_index: item_slot_index,
                                        quantity: item_slot.quantity, // we can only pick up one item at a time
                                    }
                                )
                            );
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

                        ctx.network_io.send_network_packet(
                            NetworkPacket::ItemSlotUpdate(
                                ItemSlotUpdate {
                                    area_id,
                                    player_id: self.id,
                                    inventory_index: item_slot_index,
                                    item_slot: item_slot_save,
                                }
                            )
                        );

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

    pub fn set_facing(&mut self, facing: Facing) {
        self.facing = facing
    } 

    pub fn move_camera_enemies(&mut self, ctx: &mut ClientTickContext, minimum_camera_width: f32, minimum_camera_height: f32, space: &Space, average_enemy_pos: Vector2<f32>, max_camera_y: f32) {


        let our_player_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position();

        let distance_x = (our_player_pos.translation.x - average_enemy_pos.x).abs();
        let distance_y = (our_player_pos.translation.y - average_enemy_pos.y).abs();

        
        let camera_x = our_player_pos.translation.x.min(average_enemy_pos.x) - 200.; // add some padding
        let camera_y = our_player_pos.translation.y.max(average_enemy_pos.y) + 200.;

        
        let ratio = screen_height() / screen_width();


        ctx.camera_rect.x = camera_x;
        ctx.camera_rect.y = camera_y;
        ctx.camera_rect.w = (distance_x + 400.).max(minimum_camera_width);
        ctx.camera_rect.h = (ctx.camera_rect.w) * ratio;

        // clamp max camera y pos so we dont go below the level
        let max_camera_y = max_camera_y - ctx.camera_rect.h;
        ctx.camera_rect.y = ctx.camera_rect.y.min(max_camera_y);


    }

    pub fn move_camera(&mut self, space: &Space, max_camera_y: f32, average_enemy_pos: Option<Vector2<f32>>, ctx: &mut ClientTickContext, minimum_camera_width: f32, minimum_camera_height: f32) {

        // there are two modes for the camera. One where there are enemies and one where there are none
        match average_enemy_pos {
            Some(average_enemy_pos) => {
                self.move_camera_enemies(ctx, minimum_camera_width, minimum_camera_height, space, average_enemy_pos, max_camera_y);
            },
            None => {

                let position = space.rigid_body_set.get(self.body.body_handle).unwrap().translation();

                let macroquad_position = rapier_to_macroquad(*position);

                
                if macroquad_position.x > ctx.camera_rect.right() - 200. {
                    ctx.camera_rect.x = (macroquad_position.x - ctx.camera_rect.w) + 200.;
                }

                if macroquad_position.x < ctx.camera_rect.left() + 200. {
                    
                    ctx.camera_rect.x = macroquad_position.x - 200.
                }

                if macroquad_position.y > ctx.camera_rect.bottom() - 100. {
                

                    ctx.camera_rect.y = (macroquad_position.y - ctx.camera_rect.h) + 100.;
                }

                if macroquad_position.y < ctx.camera_rect.top() + 100. {
                

                    ctx.camera_rect.y = macroquad_position.y - 100.
                }

                //let max_camera_y = max_camera_y - ctx.camera_rect.h;

                //ctx.camera_rect.y = ctx.camera_rect.y.min(max_camera_y);                
            },
        }
        


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

    pub fn new(pos: Isometry2<f32>, space: &mut Space, owner: ClientId) -> Self {
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

        let mut inventory = Inventory::new();


        // items[0] = Some(ItemSlot {
        //     quantity: 1,
        //     item: Item::Prop(
        //         PropItem {
        //             prefab_path: PathBuf::from("prefabs\\generic_physics_props\\box2.json"),
        //         }
        //     ),
        // });
        // items[3] = Some(ItemSlot {
        //     quantity: 1,
        //     item: Item::Prop(
        //         PropItem {
        //             prefab_path: PathBuf::from("prefabs\\generic_physics_props\\box2.json"),
        //         }
        //     ),
        // });

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
            last_changed_inventory_slot: Instant::now(),
            junk: Vec::new(),
            last_dash: web_time::Instant::now(),
            previous_pos: Isometry2::default(),
            last_position_update: web_time::Instant::now(),
            last_autofire: web_time::Instant::now(),
            previous_selected_item: 1,
            flying: true
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
        ctx: &mut ClientTickContext, 
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


    pub fn control(&mut self, space: &mut Space, ctx: &mut ClientTickContext) {
        let body = space.rigid_body_set.get_mut(self.body.body_handle).unwrap();

        self.jump(body);
        self.fly(body);

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
        ctx: &mut ClientTickContext, 
        space: &mut Space, 
        area_id: AreaId,
        players: &mut Vec<Player>,
        enemies: &mut Vec<Enemy>,
        props: &mut Vec<Prop>,
        bullet_trails: &mut Vec<BulletTrail>,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        dropped_items: &mut Vec<DroppedItem>,
        max_camera_y: f32,
        average_enemy_pos: Option<Vector2<f32>>,
        minimum_camera_width: f32,
        minimum_camera_height: f32,
        tiles: &mut Vec<Vec<Option<Tile>>>

    ) {

        let current_velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().vels().clone();

        self.angle_weapon_to_mouse(space, &ctx.camera_rect);
        
        self.angle_head_to_mouse(space);


        self.materialize_tiles(space, tiles);

        
        

        if self.owner == *ctx.client_id {
            self.owner_tick(space, ctx, area_id, players, enemies, props, bullet_trails, dissolved_pixels, dropped_items, max_camera_y, average_enemy_pos, minimum_camera_width, minimum_camera_height);
        }   

        self.unequip_previous_weapon(space);

        self.equip_selected_weapon(space);


        self.previous_selected_item = self.selected_item;

        self.previous_velocity = current_velocity;
        
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

    
    pub fn materialize_tiles(&mut self, space: &mut Space, tiles: &mut Vec<Vec<Tile>>) {

        let player_pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector;

        let player_pos_tile_space = Vector2::new((player_pos.x / 50.) as usize, (player_pos.y / 50.) as usize);


        // search a 10 by 10 area for blocks to materialize
        for possible_tile_x in (player_pos_tile_space.x.saturating_sub(5))..(player_pos_tile_space.x + 5) {

            if let Some(column) = tiles.get_mut(possible_tile_x) {

                for possible_tile_y in (player_pos_tile_space.y.saturating_sub(5))..(player_pos_tile_space.y + 5) {

                    if let Some(tile) = column.get_mut(possible_tile_y) {

                        tile.materialize(Vector2::new(possible_tile_x, possible_tile_y), space);
                    }
                }
            }
            
        }

        

        

        // let mut tile_count = 0;
        // for possible_tile_x in (rounded_player_pos.x - 500..rounded_player_pos.x + 500).step_by(50) {
        //     for possible_tile_y in (rounded_player_pos.y - 500..rounded_player_pos.y + 500).step_by(50) {
        //         let tile = match tiles.get(possible_tile_x as usize) {
        //             Some(column) => {
        //                 match column.get(possible_tile_y as usize) {
        //                     Some(tile) => match tile {
        //                         Some(tile) => tile,
        //                         None => continue,
        //                     },
        //                     None => continue,
        //                 }
        //             },
        //             None => continue,
        //         };

        //         tile_count += 1;
        //     }
        // }

        // dbg!(tile_count);

    }

    pub async fn draw(&self, space: &Space, textures:&mut crate::texture_loader::TextureLoader, prefabs: &Prefabs, fonts: &FontLoader, camera_rect: &Rect, tiles: &Vec<Vec<Option<Tile>>>) {
        
        let flip_x = match self.facing {
            Facing::Right => false,
            Facing::Left => true,
        };

        self.body.draw(textures, space, flip_x).await;
        self.head.draw(textures, space, flip_x).await;

        self.draw_selected_item(space, textures).await;



        self.draw_inventory(textures, space, prefabs, fonts);

        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position().translation.vector;

        draw_text(&format!("{:?}", pos), camera_rect.x + 40., camera_rect.y + 40., 20., WHITE);

    
        
        
    }

    pub async fn draw_selected_item(&self, space: &Space, textures: &mut TextureLoader) {
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
                self.facing = Facing::Right;

                ctx.network_io.send_network_packet(
                    NetworkPacket::PlayerFacingUpdate(
                        PlayerFacingUpdate { area_id: area_id, id: self.id, facing: Facing::Right }
                    )
                );
                
            }

        }

        if velocity.x < -100. {

            if !is_key_down(KeyCode::A) {
                return;
            }

            if self.facing != Facing::Left {
                self.facing = Facing::Left;

                ctx.network_io.send_network_packet(
                    NetworkPacket::PlayerFacingUpdate(
                        PlayerFacingUpdate { area_id: area_id, id: self.id, facing: Facing::Left }
                    )
                );

            }
        }
    }

    pub fn angle_weapon_to_mouse(&mut self, space: &mut Space, camera_rect: &Rect) {

        match &mut self.inventory.items[self.selected_item] {
            Some(item_slot) => match &mut item_slot.item {
                Item::Prop(prop) => {return;},
                Item::Weapon(weapon_type) => {

                    //println!("angling");
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

    pub fn jump(&mut self, body: &mut RigidBody) {

        if self.flying {
            return;
        }

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

    pub fn owner_tick(
        &mut self, 
        space: &mut Space, 
        ctx: &mut ClientTickContext, 
        area_id: AreaId,
        players: &mut Vec<Player>,
        enemies: &mut Vec<Enemy>,
        props: &mut Vec<Prop>,
        bullet_trails: &mut Vec<BulletTrail>,
        dissolved_pixels: &mut Vec<DissolvedPixel>,
        dropped_items: &mut Vec<DroppedItem>,
        max_camera_y: f32,
        average_enemy_pos: Option<Vector2<f32>>,
        minimum_camera_width: f32,
        minimum_camera_height: f32,
    ) {

        let pos = space.rigid_body_set.get(self.body.body_handle).unwrap().position();

        if (self.last_position_update.elapsed().as_secs_f32() > 3.) && *pos != self.previous_pos {


            ctx.network_io.send_network_packet(
                NetworkPacket::PlayerPositionUpdate(
                    PlayerPositionUpdate {
                        area_id,
                        pos: *pos,
                        player_id: self.id,
                    }
                )
            );

            self.last_position_update = web_time::Instant::now();
        }


        if is_mouse_button_released(macroquad::input::MouseButton::Left) {
            self.use_item(ctx, space, props, players, bullet_trails, self.facing, area_id, enemies, dissolved_pixels, dropped_items);
        }

        self.update_cursor_pos(ctx, area_id);

        self.dash(space);

        self.pickup_item(dropped_items, space, ctx, area_id);

        self.change_active_inventory_slot(ctx, area_id, space);

    

        self.change_facing_direction(space, ctx, area_id);

        self.control(space, ctx);

        let current_velocity = space.rigid_body_set.get(self.body.body_handle).unwrap().vels();

        self.move_camera(space, max_camera_y, average_enemy_pos, ctx, minimum_camera_width, minimum_camera_height);
        
        if self.previous_velocity != *current_velocity {
            ctx.network_io.send_network_packet(
                crate::updates::NetworkPacket::PlayerVelocityUpdate(
                    PlayerVelocityUpdate { 
                        id: self.id.clone(), 
                        area_id, 
                        velocity: *current_velocity
                        
                    }
                )
            );
        }
        
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSave {
    pos: Isometry2<f32>,
    owner: ClientId,
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