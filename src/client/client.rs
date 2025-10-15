use std::{collections::HashMap, path::PathBuf};

use interceptors_lib::{area::Area, bullet_trail::BulletTrail, button::Button, dropped_item::DroppedItem, enemy::Enemy, font_loader::FontLoader, player::{ItemSlot, Player}, prop::Prop, screen_shake::ScreenShakeParameters, sound_loader::SoundLoader, texture_loader::TextureLoader, updates::{NetworkPacket, Ping}, weapons::weapon_type::WeaponType, world::World, ClientIO, ClientId, ClientTickContext, Prefabs};
use macroquad::{camera::{set_camera, set_default_camera, Camera2D}, color::{BLACK, WHITE}, input::{is_key_released, KeyCode}, math::{vec2, Rect}, prelude::{gl_use_default_material, gl_use_material, load_material, Material, ShaderSource}, texture::{draw_texture_ex, render_target, DrawTextureParams, RenderTarget}, time::draw_fps, window::{clear_background, next_frame, screen_height, screen_width}};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

const CRT_FRAGMENT_SHADER: &'static str = r#"#version 100
precision lowp float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;

// https://www.shadertoy.com/view/XtlSD7

vec2 CRTCurveUV(vec2 uv)
{
    uv = uv * 2.0 - 1.0;
    vec2 offset = abs( uv.yx ) / vec2( 6.0, 4.0 );
    uv = uv + uv * offset * offset;
    uv = uv * 0.5 + 0.5;
    return uv;
}

void DrawVignette( inout vec3 color, vec2 uv )
{
    float vignette = uv.x * uv.y * ( 1.0 - uv.x ) * ( 1.0 - uv.y );
    vignette = clamp( pow( 16.0 * vignette, 0.3 ), 0.0, 1.0 );
    color *= vignette;
}


void DrawScanline( inout vec3 color, vec2 uv )
{
    float iTime = 0.1;
    float scanline 	= clamp( 0.95 + 0.05 * cos( 3.14 * ( uv.y + 0.008 * iTime ) * 240.0 * 1.0 ), 0.0, 1.0 );
    float grille 	= 0.85 + 0.15 * clamp( 1.5 * cos( 3.14 * uv.x * 640.0 * 1.0 ), 0.0, 1.0 );
    color *= scanline * grille * 1.2;
}

void main() {
    vec2 crtUV = CRTCurveUV(uv);
    vec3 res = texture2D(Texture, uv).rgb * color.rgb;
    if (crtUV.x < 0.0 || crtUV.x > 1.0 || crtUV.y < 0.0 || crtUV.y > 1.0)
    {
        res = vec3(0.0, 0.0, 0.0);
    }
    DrawVignette(res, crtUV);
    DrawScanline(res, uv);
    gl_FragColor = vec4(res, 1.0);

}
"#;

const CRT_VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

include!(concat!(env!("OUT_DIR"), "/prefabs.rs"));

pub struct Client {
    network_io: ClientIO,
    pings: HashMap<u64, web_time::Instant>,
    world: World, 
    client_id: ClientId,
    camera_rect: Rect,
    textures: TextureLoader,
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
    test_button: Button
}

impl Client {
    pub async fn connect() -> Self {

        let mut prefab_data = Prefabs::new();

        for prefab_path in PREFAB_PATHS {
            prefab_data.load_prefab_data(prefab_path).await
        }

        let url = "ws://127.0.0.1:5560";

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
                            println!("we got the opened message!");
                            break;
                        },
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                _ => panic!("received a message from the server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => panic!("received error when trying to connect to server: {}", error),
                        ewebsock::WsEvent::Closed => panic!("server closed when trying to connect"),
                        
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

        let mut textures = TextureLoader::new();

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


        let mut sounds = SoundLoader::new();

        let mut fonts = FontLoader::new();

        for asset in ASSET_PATHS {
            if asset.ends_with(".wav") {
                sounds.load(PathBuf::from(asset)).await
            }

            if asset.ends_with(".png") {
                textures.load(PathBuf::from(asset)).await;
            }

            if asset.ends_with(".ttf") {
                fonts.load(PathBuf::from(asset)).await;
            }

        }


        let test_button = Button::new(Rect {
            x: 0.,
            y: 0.,
            w: 100.,
            h: 100.,
        });


        Self {
            network_io: server,
            pings: HashMap::new(),
            world: World::empty(),
            client_id,
            camera_rect,
            textures,
            last_tick: web_time::Instant::now(),
            last_tick_duration: web_time::Duration::from_millis(1),
            latency: web_time::Duration::from_millis(1),
            last_ping_sample: web_time::Instant::now(),
            prefab_data,
            material,
            render_target: world_render_target,
            screen_shake: ScreenShakeParameters::default(None, None),
            start: web_time::Instant::now(),
            sounds,
            spawned: false,
            camera,
            fonts,
            test_button

        }
        

    }
}

impl Client {

    pub async fn run(&mut self) {

        
        loop {

            self.tick();

            let packets = self.network_io.receive_packets();

            self.handle_packets(packets);

            self.draw().await;
            
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

                    self.world.areas.push(Area::from_save(load_area.area, Some(load_area.id), &self.prefab_data));
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

                    area.props.push(Prop::from_save(update.prop, &mut area.space));


                },
                NetworkPacket::NewPlayer(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.players.push(Player::from_save(update.player, &mut area.space));
                },
                NetworkPacket::PlayerVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    let player_body = area.space.rigid_body_set.get(player.body.body_handle).unwrap();

                    let player_pos = player_body.position();

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

                    player.set_facing(update.facing);
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

                    dbg!("received pos update");
                        
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

                    prop.despawn(&mut area.space, area.id, None);


                    area.props.retain(|prop| {prop.id != update.prop_id});
                },
                NetworkPacket::NewDroppedItemUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.dropped_items.push(
                        DroppedItem::from_save(update.dropped_item, &mut area.space, &self.prefab_data)
                    );
                },
                NetworkPacket::RemoveDroppedItemUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.dropped_items.retain_mut(|dropped_item| {
                        if dropped_item.id == update.dropped_item_id {

                            dropped_item.despawn(&mut area.space);

                            false
                        } else {
                            true
                        }
                    });
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
                            dbg!("received quantity update for invalid item index");

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
                                ItemSlot::from_save(item_slot_save, &mut area.space)
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

                    enemy.despawn(&mut area.space);

                    area.enemies.retain(|enemy| {enemy.id != update.enemy_id});

                },
                NetworkPacket::EnemyHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    enemy.last_health_update = web_time::Instant::now();

                    enemy.health = update.health
                },
                NetworkPacket::PlayerHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.health = update.health
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

        //dbg!(self.test_button);

        //self.phone.update_animation();

        //self.update_camera_to_match_screen_size();

        self.measure_latency();

        // if is_key_released(KeyCode::E) {
        //     self.camera_rect.w *= 1.2;
        //     self.camera_rect.h *= 1.2;
        // }

        // if is_key_released(KeyCode::Q) {
        //     self.camera_rect.w *= 0.8;
        //     self.camera_rect.h *= 0.8;
        // }

        self.ping();

        let mut ctx = ClientTickContext {
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

        self.world.client_tick(&mut ctx);

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

    
    pub async fn draw(&mut self) { 

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


        self.world.draw(&mut self.textures, &self.camera_rect, &self.prefab_data, &self.camera, &self.fonts, self.start.elapsed()).await;

        //self.phone.draw(&self.textures, &self.camera_rect);
        

        set_default_camera();

        gl_use_material(&self.material);



        draw_texture_ex(&self.render_target.texture, 0.0, 0., WHITE, DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            ..Default::default()
        });

        gl_use_default_material();

        draw_fps();
        
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

