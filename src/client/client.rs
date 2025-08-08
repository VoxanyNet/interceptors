use std::{collections::HashMap, pin::Pin, time::Instant};

use ewebsock::{WsReceiver, WsSender};
use interceptors_lib::{area::Area, texture_loader::TextureLoader, updates::{NetworkPacket, Ping}, world::World, ClientIO, ClientId, ClientTickContext};
use ldtk2::{Ldtk, LdtkJson};
use macroquad::{camera::{set_camera, set_default_camera, Camera2D}, color::WHITE, file::{load_file, load_string}, input::{is_key_released, mouse_wheel, KeyCode}, math::{Rect, Vec2},  texture::{draw_texture_ex, load_texture, DrawTextureParams, Texture2D}, time::draw_fps, window::next_frame};
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


        let camera_rect = Rect::new(0., 0., 480., 320.);

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

            self.receive_packets();

            self.draw().await
        }
    }

    pub fn receive_packets(&mut self) {
        loop {
            let network_packet_bytes = match self.network_io.receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => todo!("unhandled 'Opened' event"),
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                ewebsock::WsMessage::Binary(bytes) => bytes,
                                _ => todo!("unhandled message type when trying to receive packet from server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => {

                            // this is stupid
                            if error.contains("A non-blocking socket operation could not be completed immediately)") {
                                println!("io error: {}", error);
                                return;
                            }
                            todo!("unhandled 'Error' event when trying to receive update from server: {}", error)
                        },
                        ewebsock::WsEvent::Closed => todo!("server closed"),
                    }
                },
                None => break, // this means there are no more updates
            };

            let network_packet: NetworkPacket = bitcode::deserialize(&network_packet_bytes).unwrap();

            self.handle_packet(network_packet);

        }
    }

    pub fn handle_packet(&mut self, packet: NetworkPacket) {
        match packet {
            NetworkPacket::Ping(ping) => {
                let time = self.pings.get(&ping.id).unwrap().elapsed().as_micros();

                dbg!(time);

            },
            _ => {
                println!("unhandled network packet")
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

        dbg!(self.camera_rect);

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

        self.world.draw(&mut self.textures).await;


        set_default_camera();
        

        draw_fps();

        next_frame().await
    }
}

