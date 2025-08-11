use std::{collections::HashMap, fmt::Display, net::{TcpListener, TcpStream}};

use ewebsock::{WsReceiver, WsSender};
use macroquad::{camera::Camera2D, color::{Color, WHITE}, input::{is_key_down, is_key_released, mouse_position, KeyCode}, math::{vec2, Rect, Vec2}, shapes::DrawRectangleParams, texture::{draw_texture_ex, DrawTextureParams}, window::{get_internal_gl, screen_height}};
use nalgebra::geometry;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use tungstenite::WebSocket;
use uuid::Uuid;

use crate::{all_keys::ALL_KEYS, space::Space, texture_loader::TextureLoader, updates::NetworkPacket};

pub mod space;
pub mod updates;
pub mod area;
pub mod texture_loader;
pub mod world;
pub mod decoration;
pub mod player;
pub mod clip;
pub mod background;
pub mod generic_physics_prop;
pub mod all_keys;

pub fn is_key_down_exclusive(required: &[KeyCode]) -> bool {
    // All required keys must be down
    if !required.iter().all(|&k| is_key_down(k)) {
        return false;
    }

    // No other keys must be down
    for &key in ALL_KEYS.iter() {
        if !required.contains(&key) && is_key_down(key) {
            return false;
        }
    }

    true
}

pub fn is_key_released_exclusive(required: &[KeyCode]) -> bool {
    // All required keys must be down
    if !required.iter().all(|&k| is_key_released(k)) {
        return false;
    }

    // No other keys must be down
    for &key in ALL_KEYS.iter() {
        if !required.contains(&key) && (is_key_down(key) || is_key_released(key)) {
            return false;
        }
    }

    true
}
pub async fn draw_texture_onto_physics_body(
    rigid_body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    space: &Space, 
    texture_path: &String, 
    textures: &mut TextureLoader, 
    flip_x: bool, 
    flip_y: bool, 
    additional_rotation: f32
) {
    let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();
    let collider = space.collider_set.get(collider_handle).unwrap();

    // use the shape to define how large we should draw the texture
    // maybe we should change this
    let shape = collider.shape().as_cuboid().unwrap();

    let position = rigid_body.position().translation;
    let body_rotation = rigid_body.rotation().angle();

    let draw_pos = rapier_to_macroquad(&vec2(position.x, position.y));

    draw_texture_ex(
        textures.get(texture_path).await, 
        draw_pos.x - shape.half_extents.x, 
        draw_pos.y - shape.half_extents.y, 
        WHITE, 
        DrawTextureParams {
            dest_size: Some(vec2(shape.half_extents.x * 2., shape.half_extents.y * 2.)),
            source: None,
            rotation: (body_rotation * -1.) + additional_rotation,
            flip_x,
            flip_y,
            pivot: None,
        }
    );

    
}

pub fn draw_hitbox(space: &Space, rigid_body_handle: RigidBodyHandle, collider_handle: ColliderHandle, color: Color) {
    let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();
    let collider = space.collider_set.get(collider_handle).unwrap();

    let shape = collider.shape().as_cuboid().unwrap();

    let position = collider.position().translation;
    let rotation = rigid_body.rotation().angle();

    let draw_pos = rapier_to_macroquad(&vec2(position.x, position.y));

    macroquad::shapes::draw_rectangle_ex(
        draw_pos.x,
        draw_pos.y, 
        shape.half_extents.x * 2., 
        shape.half_extents.y * 2., 
        DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color }
    );

}


pub fn mouse_world_pos(camera_rect: &Rect) -> Vec2 {
    let mouse_pos = mouse_position();

    let mut camera = Camera2D::from_display_rect(*camera_rect);
    camera.zoom.y = -camera.zoom.y;

    camera.screen_to_world(mouse_pos.into())

}

pub fn rapier_mouse_world_pos(camera_rect: &Rect) -> Vec2 {
    macroquad_to_rapier(
        &mouse_world_pos(camera_rect)
    )
}

pub struct ClientIO {
    pub send: WsSender,
    pub receive: WsReceiver,
}


impl ClientIO {
    pub fn send_network_packet(&mut self, packet: NetworkPacket) {
        self.send.send(
            ewebsock::WsMessage::Binary(
                bitcode::serialize(&packet).unwrap()
            )
        );
    }

    pub fn receive_packets(&mut self) -> Vec<NetworkPacket> {

        let mut packets: Vec<NetworkPacket> = Vec::new();

        loop {
            let network_packet_bytes = match self.receive.try_recv() {
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
                                return Vec::new();
                            }
                            todo!("unhandled 'Error' event when trying to receive update from server: {}", error)
                        },
                        ewebsock::WsEvent::Closed => todo!("server closed"),
                    }
                },
                None => break, // this means there are no more updates
            };

            let network_packet: NetworkPacket = bitcode::deserialize(&network_packet_bytes).unwrap();

            packets.push(network_packet);

        }

        packets
    }

}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ClientId {
    id: u64
}
impl ClientId {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u64_pair().0
        }
    }
}
pub struct ServerIO {
    pub clients: HashMap<ClientId, WebSocket<TcpStream>>,
    pub listener: TcpListener,
}

impl ServerIO {
    pub fn new() -> Self {
        let listener = match TcpListener::bind("127.0.0.1:5560") {
            Ok(listener) => listener,
            Err(error) => panic!("failed to bind listener: {}", error),
        };

        match listener.set_nonblocking(true) {
            Ok(_) => {},
            Err(error) => panic!("failed to set server as non blocking: {}", error),
        };

        Self {
            clients: HashMap::new(),
            listener,
        }
    }

    // WE NEED TO ADD CLIENT IDs SO THAT WE CAN SPECIFY
    pub fn send_all_clients(&mut self) {
        for client in &mut self.clients {

        }
    }

    pub fn send_client(&mut self, client_id: ClientId, packet: NetworkPacket) {
        self.clients.get_mut(&client_id).unwrap().send(
            tungstenite::Message::Binary(
                bitcode::serialize(&packet).unwrap().into()
            )
        ).unwrap();
    }

    pub fn accept_new_client(&mut self) -> Option<ClientId> {
        match self.listener.accept() {
            Ok((stream, address)) => {
                println!("received new connection from address: {}", address);

                stream.set_nonblocking(true).expect("Failed to set new client as non blocking");

                let mut websocket_stream = loop {
                    match tungstenite::accept(stream.try_clone().expect("failed to clone stream")) {
                        Ok(websocket_stream) => break websocket_stream,
                        Err(error) => {
                            match error {
                                tungstenite::HandshakeError::Interrupted(_) => continue, // try again if the handshake isnt done yet
                                tungstenite::HandshakeError::Failure(error) => panic!("handshake failed with new client: {}", error),
                            }
                        },
                    };
                };

                let client_id: ClientId = loop {
                    match websocket_stream.read() {
                        Ok(message) => {
                            match message {
                                tungstenite::Message::Binary(bytes) => {
                                    break bitcode::deserialize(&bytes).unwrap()
                                },
                                _ => {
                                    panic!("client tried to send non binary data when sending client id")
                                }
                            }
                        },
                        Err(error) => {
                            match error {
                                
                                tungstenite::Error::Io(error) => {
                                    match error.kind() {
                                        std::io::ErrorKind::WouldBlock => {
                                            // keep waiting until the client sends the client id
                                            continue;
                                        },
                                        _ => {
                                            panic!("an error occured while reading the client's id: {}", error)
                                        },
                                    }
                                },
                                _ => {
                                    panic!("an error occured while reading the client's id: {}", error)
                                }
                                
                            }
                        },
                    }
                };
                
            
                println!("new client connected!");

                self.clients.insert(client_id, websocket_stream);

                return Some(client_id)

            },
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::WouldBlock => return None, // no new clients

                    _ => {
                        println!("Something went wrong trying to accept a new client");
                        return None
                    }
                }
            },
        }
    }

}

pub struct ClientTickContext<'a> {
    pub network_io: &'a mut ClientIO
}

pub fn macroquad_to_rapier(macroquad_coords: &Vec2) -> Vec2 {

    // translate macroquad coords to rapier coords
    Vec2 { 
        x: macroquad_coords.x, 
        y: (macroquad_coords.y * -1.) + screen_height()
    }
}

pub fn rapier_to_macroquad(rapier_coords: &Vec2) -> Vec2 {
    Vec2 {
        x: rapier_coords.x,
        y: (rapier_coords.y * -1.) + screen_height()
    }
}