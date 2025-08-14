use std::{cmp::max, collections::HashMap, pin::Pin, time::{Duration, Instant}};

use ewebsock::{WsReceiver, WsSender};
use interceptors_lib::{area::Area, mouse_world_pos, player::Player, prop::Prop, texture_loader::TextureLoader, updates::{NetworkPacket, Ping}, world::World, ClientIO, ClientId, ClientTickContext};
use ldtk2::{Ldtk, LdtkJson};
use macroquad::{camera::{set_camera, set_default_camera, Camera2D}, color::{LIGHTGRAY, WHITE}, file::{load_file, load_string}, input::{is_key_down, is_key_released, mouse_position, mouse_wheel, KeyCode}, math::{vec2, Rect, Vec2}, prelude::{gl_use_default_material, gl_use_material, load_material, ShaderSource}, texture::{draw_texture_ex, load_texture, render_target, DrawTextureParams, Texture2D}, time::draw_fps, ui::root_ui, window::{clear_background, next_frame, screen_height, screen_width}};
use macroquad_tiled::{load_map, Map};
use uuid::Uuid;

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

pub struct Client {
    network_io: ClientIO,
    pings: HashMap<u64, Instant>,
    world: World, 
    client_id: ClientId,
    camera_rect: Rect,
    textures: TextureLoader,
    last_tick_duration: Duration,
    last_tick: Instant,
    latency: Duration,
    last_ping_sample: Instant
}

impl Client {
    pub async fn connect() -> Self {

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

        let server = ClientIO {
            send: server_send,
            receive: server_receive,
            packet_queue: Vec::new()
        };

        let textures = TextureLoader::new();


        let camera_rect = Rect {
            x: 0.,
            y: 0.,
            w: 1280.,
            h: 720.,
        };

        Self {
            network_io: server,
            pings: HashMap::new(),
            world: World::empty(),
            client_id,
            camera_rect,
            textures,
            last_tick: Instant::now(),
            last_tick_duration: Duration::from_millis(1),
            latency: Duration::from_millis(1),
            last_ping_sample: Instant::now() - Duration::from_secs(10)

        }
        

    }
}

impl Client {

    pub async fn run(&mut self) {

        let render_target = render_target(1280, 720);

        render_target.texture.set_filter(macroquad::texture::FilterMode::Nearest);

        let material = load_material(
            ShaderSource::Glsl {
                vertex: CRT_VERTEX_SHADER,
                fragment: CRT_FRAGMENT_SHADER,
            },
            Default::default(),
        )
        .unwrap();

        
        loop {

            self.tick();

            let packets = self.network_io.receive_packets();

            self.handle_packets(packets);

            let mut camera = Camera2D::from_display_rect(self.camera_rect);
            
            camera.render_target = Some(render_target.clone());

            camera.zoom.y = -camera.zoom.y;

            // let camera = &Camera2D{
            //         zoom: vec2(1., 1.),
            //         target: vec2(0.0, 0.0),
            //         render_target: Some(render_target.clone()),
            //         ..Default::default()
            // };

            

            set_camera(
                &camera
            );
            

            self.draw().await;

            set_default_camera();

            gl_use_material(&material);

            draw_texture_ex(&render_target.texture, 0.0, 0., WHITE, DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            });

            gl_use_default_material();

            next_frame().await;


            
        }
    }

    pub fn measure_latency(&mut self) {

        let ping = Ping::new();
        // sample ping every second
        if self.last_ping_sample.elapsed().as_secs_f32() > 1. {
            self.network_io.send_network_packet(NetworkPacket::Ping(ping));
        };

        self.pings.insert(ping.id.clone(), Instant::now());

    }

    pub fn handle_packets(&mut self, packets: Vec<NetworkPacket>) {

        for packet in packets {
            match packet {
            NetworkPacket::Ping(ping) => {
                self.latency = self.pings.remove(&ping.id).unwrap().elapsed();

            },
            NetworkPacket::LoadArea(load_area) => {

                self.world.areas.push(Area::from_save(load_area.area, Some(load_area.id)));
            }

            NetworkPacket::PropVelocityUpdate(update) => {
                let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id} ).unwrap();

                let prop = area.props.iter_mut().find(|prop| {prop.id == update.id}).unwrap();

                prop.set_velocity(update.velocity, &mut area.space);
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
            }
            _ => {
                println!("unhandled network packet")
            }
        }
        }
        
    }

    pub fn ping(&mut self) {
        if is_key_released(KeyCode::E) {

            let ping = Ping::new();

            self.pings.insert(ping.id, Instant::now());

            self.network_io.send_network_packet(
                NetworkPacket::Ping(ping)
            );
        }
    }
    pub fn tick(&mut self) {

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
            camera_rect: &mut self.camera_rect
        };

        self.world.client_tick(&mut ctx);

        self.network_io.flush();
        
        self.last_tick_duration = self.last_tick.elapsed();
        self.last_tick = Instant::now();
    }
    pub async fn draw(&mut self) { 


        self.world.draw(&mut self.textures, &self.camera_rect).await;

    }
}

