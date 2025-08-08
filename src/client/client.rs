use std::{collections::HashMap, pin::Pin, time::Instant};

use ewebsock::{WsReceiver, WsSender};
use interceptors_lib::{area::Area, texture_loader::TextureLoader, updates::{NetworkPacket, Ping}, world::World, ClientIO, ClientId, ClientTickContext};
use ldtk2::{Ldtk, LdtkJson};
use macroquad::{camera::{set_camera, set_default_camera, Camera2D}, color::WHITE, file::{load_file, load_string}, input::{is_key_released, mouse_wheel, KeyCode}, math::{Rect, Vec2}, texture::{draw_texture_ex, load_texture, DrawTextureParams, Texture2D}, time::draw_fps, ui::root_ui, window::next_frame};
use macroquad_tiled::{load_map, Map};


pub struct Client {
    network_io: ClientIO,
    // when we sent this ping id
    pings: HashMap<u64, Instant>,
    world: World,
    client_id: ClientId,
    camera_rect: Rect,
    textures: TextureLoader
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
            textures
        }
        

    }
}

impl Client {

    pub async fn run(&mut self) {
        loop {
            self.tick();

            let packets = self.network_io.receive_packets();

            self.handle_packets(packets);

            self.draw().await
        }
    }

    pub fn handle_packets(&mut self, packets: Vec<NetworkPacket>) {

        for packet in packets {
            match packet {
            NetworkPacket::Ping(ping) => {
                let time = self.pings.get(&ping.id).unwrap().elapsed().as_micros();

                dbg!(time);

            },
            NetworkPacket::LoadLobby(load_lobby) => {

                self.world.lobby = Area::from_save(load_lobby.area)
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

        if is_key_released(KeyCode::E) {
            self.camera_rect.w *= 1.2;
            self.camera_rect.h *= 1.2;
        }

        if is_key_released(KeyCode::Q) {
            self.camera_rect.w *= 0.8;
            self.camera_rect.h *= 0.8;
        }

        self.ping();

        let mut ctx = ClientTickContext {
            network_io: &mut self.network_io,
        };

        self.world.client_tick(&mut ctx);
    }
    pub async fn draw(&mut self) {  

        let mut camera = Camera2D::from_display_rect(self.camera_rect);
        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

        self.world.draw(&mut self.textures, &self.camera_rect).await;


        set_default_camera();
        

        draw_fps();

        next_frame().await
    }
}

