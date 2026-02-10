use std::{collections::HashMap, process::exit};

use interceptors_lib::{Assets, ClientIO, ClientId, ClientTickContext, Prefabs, area::Area, bullet_trail::BulletTrail, button::Button, dropped_item::DroppedItem, enemy::Enemy, font_loader::FontLoader, get_intersections, material_loader::MaterialLoader, player::{ItemSlot, Player}, prop::Prop, screen_shake::ScreenShakeParameters, sound_loader::SoundLoader, texture_loader::ClientTextureLoader, updates::{NetworkPacket, Ping}, weapons::weapon_type::WeaponType, world::World};
use macroquad::{camera::{Camera2D, set_camera, set_default_camera}, color::{BLACK, WHITE}, input::{KeyCode, is_key_released, show_mouse}, math::{Rect, vec2}, prelude::{Material, ShaderSource, gl_use_default_material, load_material}, text::draw_text, texture::{DrawTextureParams, RenderTarget, draw_texture_ex, render_target}, time::draw_fps, window::{clear_background, next_frame, screen_height, screen_width}};
use rapier2d::math::Vector;

use crate::{shaders::{CRT_FRAGMENT_SHADER, CRT_VERTEX_SHADER}};


pub struct Client {
    network_io: ClientIO,
    pings: HashMap<u64, web_time::Instant>,
    world: World, 
    client_id: ClientId,
    camera_rect: Rect,
    textures: ClientTextureLoader,
    last_tick_duration: web_time::Duration,
    last_tick: web_time::Instant,
    latency: web_time::Duration,
    last_ping_sample: web_time::Instant,
    prefab_data: Prefabs,
    material: Material,
    render_target: RenderTarget,
    screen_shake: ScreenShakeParameters,
    start: web_time::Instant,
    sounds: SoundLoader,
    camera: Camera2D,
    spawned: bool,
    fonts: FontLoader,
    test_button: Button,
    material_loader: MaterialLoader
}

impl Client {
    pub async fn connect(assets: Assets, _client_id: i64) -> Self {


        show_mouse(true);
        
        let url = "ws://127.0.0.1:5560";

        #[cfg(target_arch = "wasm32")]
        let url = "wss://interceptors.voxany.net/ws/";

        #[cfg(feature = "discord")]
        let url = format!("wss://{}.discordsays.com/ws/", client_id);

        let (mut server_send, server_receive) = match ewebsock::connect(url, ewebsock::Options::default()) {
            Ok(result) => result,
            Err(error) => {
                panic!("failed to connect to server: {}", error)
            },
        };

        // wait for Opened event from server
        loop {
            match server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => {
                            log::info!("Received opened message from server");
                            break;
                        },
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                _ => {
                                    log::error!("Received a message from the server");
                                    exit(1);
                                }
                            }
                        },
                        ewebsock::WsEvent::Error(error) => {
                            log::error!("Received error when trying to connect to server: {}", error);
                            exit(1);
                        },
                        ewebsock::WsEvent::Closed => {
                            log::error!("Server closed when trying to connect");
                            exit(1);
                        }
                        
                    }
                },
                None => {
                    
                    macroquad::window::next_frame().await; // let js runtime main thread continue execution while we wait

                    continue;
                },
            }
        };

        let client_id = ClientId::new();

        server_send.send(
            ewebsock::WsMessage::Binary(
                bitcode::serialize(&client_id).unwrap()
            )
        );

        let world_render_target = render_target(1280, 720);

        world_render_target.texture.set_filter(macroquad::texture::FilterMode::Nearest);

        let material = load_material(
            ShaderSource::Glsl {
                vertex: CRT_VERTEX_SHADER,
                fragment: CRT_FRAGMENT_SHADER,
            },
            Default::default(),
        ).unwrap();

        let server = ClientIO {
            send: server_send,
            receive: server_receive,
            packet_queue: Vec::new()
        };

        // create world camera
        let camera_rect = Rect {
            x: 0.,
            y: 0.,
            w: 1280.,
            h: 720.,
        };

        let mut camera = Camera2D::from_display_rect(camera_rect);
        camera.render_target = Some(world_render_target.clone());

        camera.zoom.y = -camera.zoom.y;

        set_camera(
            &camera
        );   

        let test_button = Button::new(Rect {
            x: 0.,
            y: 0.,
            w: 100.,
            h: 100.,
        }, None);

        Self {
            network_io: server,
            pings: HashMap::new(),
            world: World::empty(),
            client_id,
            camera_rect,
            textures: assets.textures,
            last_tick: web_time::Instant::now(),
            last_tick_duration: web_time::Duration::from_millis(1),
            latency: web_time::Duration::from_millis(1),
            last_ping_sample: web_time::Instant::now(),
            prefab_data: assets.prefabs,
            material,
            render_target: world_render_target,
            screen_shake: ScreenShakeParameters::default(None, None),
            start: web_time::Instant::now(),
            sounds: assets.sounds,
            spawned: false,
            camera,
            fonts: assets.fonts,
            test_button,
            material_loader: assets.material_loader
        }
        

    }
}

impl Client {

    pub async fn run(&mut self) {

        
        loop {

            let then = web_time::Instant::now();
            self.tick();

            let packets = self.network_io.receive_packets();

            self.handle_packets(packets);

            self.draw(then).await;

            
            
        }
    }

    pub fn measure_latency(&mut self) {

        let ping = Ping::new();
        // sample ping every second
        if self.last_ping_sample.elapsed().as_secs_f32() > 1. {
            self.network_io.send_network_packet(NetworkPacket::Ping(ping));
        };

        self.pings.insert(ping.id.clone(), web_time::Instant::now());

    }

    pub fn handle_packets(&mut self, packets: Vec<NetworkPacket>) {

        for packet in packets {
            match packet {

                
                NetworkPacket::MasterUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.master = Some(update.master.clone());
                },
                
                NetworkPacket::Ping(ping) => {
                    self.latency = self.pings.remove(&ping.id).unwrap().elapsed();

                },
                NetworkPacket::LoadArea(load_area) => {

                    self.world.areas.push(Area::from_save(load_area.area, Some(load_area.id), &self.prefab_data, (&self.textures).into()));
                }

                NetworkPacket::PropVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id} ).unwrap();

                    let prop = area.props.iter_mut().find(|prop| {prop.id == update.id});

                    if let Some(prop) = prop { prop.set_velocity(update.velocity, &mut area.space) }
                },
                NetworkPacket::PropUpdateOwner(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = area.props.iter_mut().find(|prop| {prop.id} == update.id).unwrap();

                    prop.owner = update.owner;

                },
                NetworkPacket::NewProp(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.props.push(Prop::from_save(update.prop, &mut area.space, (&self.textures).into()));


                },
                NetworkPacket::NewPlayer(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.players.push(Player::from_save(update.player, &mut area.space, (&self.textures).into()));
                },
                NetworkPacket::PlayerVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    let player_body = area.space.rigid_body_set.get(player.body.body_handle).unwrap();

                    let _player_pos = player_body.position();

                    player.set_velocity(update.velocity, &mut area.space);
                },
                NetworkPacket::PlayerCursorUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    player.set_cursor_pos(update.pos);
                },
                NetworkPacket::PlayerFacingUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    player.facing = update.facing;
                },
                NetworkPacket::SpawnBulletTrail(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.bullet_trails.push(
                        BulletTrail::from_save(update.save)
                    );
                },
                NetworkPacket::PlayerPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();
                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();
                    player.set_pos(update.pos, &mut area.space);



                    
                },
                NetworkPacket::PropPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = if let Some(prop) = area.props.iter_mut().find(|prop| {prop.id} == update.prop_id) {prop} else {continue};

                    let current_pos = match area.space.rigid_body_set.get(prop.rigid_body_handle) {
                        Some(body) => body.position(),
                        None => {
                            continue;
                        },
                    };

                    if (update.pos.translation.x - current_pos.translation.x).abs() > 4. {
                        prop.set_pos(update.pos, &mut area.space);
                    }

                },
                NetworkPacket::DissolveProp(update) => {

                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = area.props.iter_mut().find(|prop|{prop.id == update.prop_id}).unwrap();

                    prop.dissolve(&self.textures, &mut area.space, &mut area.dissolved_pixels,None, area.id);
                }
                NetworkPacket::RemovePropUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = area.props.iter_mut().find(|prop|{prop.id == update.prop_id}).unwrap();

                    prop.mark_despawn();

                },
                NetworkPacket::NewDroppedItemUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.dropped_items.push(
                        DroppedItem::from_save(update.dropped_item, &mut area.space, &self.prefab_data, (&self.textures).into())
                    );
                },
                NetworkPacket::RemoveDroppedItemUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let dropped_item = area.dropped_items.iter_mut().find(|dropped_item| {dropped_item.id == update.dropped_item_id}).unwrap();
                    
                    dropped_item.mark_despawn();
                }
                NetworkPacket::DroppedItemVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let dropped_item = area.dropped_items.iter_mut().find(|dropped_item| {dropped_item.id == update.id}).unwrap();

                    dropped_item.set_velocity(&mut area.space, update.velocity);
                },
                NetworkPacket::ActiveItemSlotUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.selected_item = update.active_item_slot as usize;

                },
                NetworkPacket::ItemSlotQuantityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    match &mut player.inventory.items[update.inventory_index] {
                        Some(item_slot) => {
                            item_slot.quantity = update.quantity;
                        },
                        None => {
                            log::warn!("Received quantity update for invalid item index");
                            continue;
                        },
                    }

                },
                NetworkPacket::ItemSlotUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.inventory.items[update.inventory_index] = match update.item_slot {
                        Some(item_slot_save) => {
                            Some(
                                ItemSlot::from_save(item_slot_save, &mut area.space, (&self.textures).into())
                            )
                        },
                        None => None,
                    }
                },

                NetworkPacket::EnemyPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    area.space.rigid_body_set.get_mut(enemy.body.body_handle).unwrap().set_position(update.position, true);
                },

                NetworkPacket::EnemyVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    area.space.rigid_body_set.get_mut(enemy.body.body_handle).unwrap().set_vels(update.velocity, true);


                },
                NetworkPacket::EnemyWeaponUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();



                    enemy.weapon = Some(WeaponType::from_save(&mut area.space, update.weapon, Some(enemy.body.body_handle)));


                },
                NetworkPacket::NewEnemyUpdate(update) => {

                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();


                    let enemy = Enemy::from_save(update.enemy, &mut area.space);

                    area.enemies.push(enemy);
                }
                NetworkPacket::EnemyDespawnUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    enemy.mark_despawn();


                },
                NetworkPacket::EnemyHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    if let Some(enemy) = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}) {
                        enemy.last_health_update = web_time::Instant::now();
                        enemy.health = update.health
                    }
                    
                },
                NetworkPacket::PlayerHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.health = update.health
                },
                NetworkPacket::PlayerDespawnUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();
                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.mark_despawn();
                },
                NetworkPacket::StupidDissolvedPixelVelocityUpdate(update) => {
                    
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let intersections = get_intersections(
                        update.weapon_pos, 
                        &mut area.space, 
                        update.bullet_vector, 
                        None
                    ); 

    

                    for dissolved_pixel in &mut area.dissolved_pixels {

                        let collider = dissolved_pixel.collider;
                        
                        for impact in intersections.iter().filter(|impact| {impact.intersected_collider == collider}) {

                            
                            let body = area.space.rigid_body_set.get_mut(dissolved_pixel.body).unwrap();
                            body.apply_impulse(
                                Vector::new(impact.intersection_vector.x * 5000., impact.intersection_vector.y * 5000.), 
                                true
                            );
                        }
                    } 

                }
            }
        }
        
    }

    pub fn ping(&mut self) {
        if is_key_released(KeyCode::E) {

            let ping = Ping::new();

            self.pings.insert(ping.id, web_time::Instant::now());

            self.network_io.send_network_packet(
                NetworkPacket::Ping(ping)
            );
        }
    }


    pub fn update_camera_to_match_screen_size(&mut self) {
        self.camera_rect.w = screen_width();
        self.camera_rect.h = screen_height();
    }
    

    pub fn phone(&mut self) {
        // if is_key_released(KeyCode::L) {
        //     self.phone.toggle();
        // }
    }

    pub fn tick(&mut self) {

        self.phone();
        self.measure_latency();
        self.ping();

        let ctx = ClientTickContext {
            network_io: &mut self.network_io,
            last_tick_duration: &self.last_tick_duration,
            client_id: &self.client_id,
            camera_rect: &mut self.camera_rect,
            prefabs: &self.prefab_data,
            screen_shake: &mut self.screen_shake,
            sounds: &mut self.sounds,
            textures: &self.textures,
            camera: &self.camera
            
        };

        // if !self.spawned {
        //     self.world.areas[0].spawn_player(&mut ctx);

        //     self.spawned = true;
        // }

        self.world.tick(&mut interceptors_lib::TickContext::Client(ctx));

        self.network_io.flush();
        
        self.last_tick_duration = self.last_tick.elapsed();
        self.last_tick = web_time::Instant::now();
    }

    pub fn calculate_shaken_camera_rect(&self) -> Rect {

        let elapsed = self.start.elapsed().as_secs_f64();

        let x_shake = {
            let frequency_modifier = self.screen_shake.x_frequency;
            
            let magnitude_modifier = self.screen_shake.x_intensity;

            let offset = self.screen_shake.x_offset;

            magnitude_modifier * ((frequency_modifier * elapsed) + offset).sin()

            
        };

        let y_shake = {
            let frequency_modifier = self.screen_shake.y_frequency;
            
            let magnitude_modifier = self.screen_shake.y_intensity;

            let offset = self.screen_shake.y_offset;

            magnitude_modifier * ((frequency_modifier * elapsed) + offset).sin()
        };

        
        // add shake
        Rect {
            x: self.camera_rect.x + x_shake as f32,
            y: self.camera_rect.y + y_shake as f32,
            w: self.camera_rect.w,
            h: self.camera_rect.h,
        }
    }

    
    pub async fn draw(&mut self, then: web_time::Instant) { 

        let shaken_camera_rect = self.calculate_shaken_camera_rect();

        // update the camera with new camera rect
        let mut camera = Camera2D::from_display_rect(shaken_camera_rect);

        self.apply_screen_shake_decays();
            
        
        camera.render_target = Some(self.render_target.clone());

        camera.zoom.y = -camera.zoom.y;

        self.camera = camera;

        set_camera(
            &self.camera
        );   

        clear_background(BLACK);

        self.world.draw(
            &mut self.textures, 
            &self.material_loader,
            &self.camera_rect, 
            &self.prefab_data, 
            &self.camera, 
            &self.fonts, 
            self.start.elapsed()
        ).await;

        //self.phone.draw(&self.textures, &self.camera_rect);
                
        set_default_camera();

        //gl_use_material(&self.material);

        draw_texture_ex(&self.render_target.texture, 0.0, 0., WHITE, DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            ..Default::default()
        });

        gl_use_default_material();

        draw_fps();
        

        draw_text(&format!("Tick time: {:?}", then.elapsed()), 0., 60., 20., WHITE);

        next_frame().await;

       


        

    }

    pub fn apply_screen_shake_decays(&mut self) {
        // apply decays
        let x_frequency_decay = self.screen_shake.x_frequency_decay * self.last_tick_duration.as_secs_f64();
        let y_frequency_decay = self.screen_shake.y_frequency_decay * self.last_tick_duration.as_secs_f64();

        let x_intensity_decay = self.screen_shake.x_intensity_decay * self.last_tick_duration.as_secs_f64();
        let y_intensity_decay = self.screen_shake.y_intensity_decay * self.last_tick_duration.as_secs_f64();

        self.screen_shake.x_frequency = (self.screen_shake.x_frequency - x_frequency_decay).max(0.0);
        self.screen_shake.y_frequency = (self.screen_shake.y_frequency - y_frequency_decay).max(0.0);

        self.screen_shake.x_intensity = (self.screen_shake.x_intensity - x_intensity_decay).max(0.0);
        self.screen_shake.y_intensity = (self.screen_shake.y_intensity - y_intensity_decay).max(0.0);
    }
}

