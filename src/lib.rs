use std::{collections::HashMap, fs::read_to_string, net::{TcpListener, TcpStream}, path::PathBuf};

use ewebsock::{WsReceiver, WsSender};
use macroquad::{camera::Camera2D, color::{Color, WHITE}, file::load_string, input::{is_key_down, is_key_released, mouse_position, KeyCode}, math::{vec2, Rect, Vec2}, prelude::camera::mouse::Camera, shapes::DrawRectangleParams, texture::{draw_texture_ex, DrawTextureParams}};
use nalgebra::Vector2;
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, QueryFilter, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use tungstenite::WebSocket;
use nalgebra::point;

use crate::{all_keys::ALL_KEYS, screen_shake::ScreenShakeParameters, sound_loader::SoundLoader, space::Space, texture_loader::TextureLoader, updates::NetworkPacket};

pub mod space;
pub mod updates;
pub mod area;
pub mod texture_loader;
pub mod world;
pub mod decoration;
pub mod player;
pub mod clip;
pub mod background;
pub mod prop;
pub mod all_keys;
pub mod body_part;
pub mod weapon;
pub mod shotgun;
pub mod bullet_trail;
pub mod screen_shake;
pub mod sound_loader;
pub mod enemy;
pub mod collider_groups;
pub mod phone;
pub mod computer;
pub mod font_loader;
pub mod button;
pub mod dropped_item;

pub struct SwapIter<'a, T> {
    vec: &'a mut Vec<T>,
    index: usize
}

impl<'a, T> SwapIter<'a, T> {
    pub fn new(collection: &'a mut Vec<T>) -> Self {
        Self {
            vec: collection,
            index: 0,
        }
    }

    pub fn next(&mut self) -> (&mut Vec<T>, T) {
        let element = self.vec.swap_remove(self.index);

        (&mut self.vec, element)

        // dont increment to the next index because we just removed an element which implicity "increments" the index
    }

    pub fn restore(&mut self, element: T) {
        
        // return the element to the vector
        self.vec.push(element);

        // swap the restored element back to its original position
        let len = self.vec.len();
        self.vec.swap(len - 1, self.index);

        self.index += 1;
    }

    pub fn not_done(&self) -> bool {
        self.index < self.vec.len()
    }
}


pub fn collider_from_texture_size(texture_size: Vec2) -> ColliderBuilder {
    ColliderBuilder::cuboid(texture_size.x / 2., texture_size.y / 2.)
}

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

pub fn rapier_to_macroquad(rapier_coords: Vector2<f32>) -> Vec2 {
    Vec2 {
        x: rapier_coords.x,
        y: (rapier_coords.y * -1.) + 720.
    }
}

pub fn uuid_string() -> String {
 
    let mut buf = [0u8; 4];
    getrandom::getrandom(&mut buf).unwrap();
    u32::from_be_bytes(buf).to_string()

}

pub fn uuid_u64() -> u64 {
 
    // WTF
    let mut buf = [0u8; 8];
    getrandom::getrandom(&mut buf).unwrap();
    u64::from_be_bytes(buf)

}

pub async fn draw_texture_onto_physics_body(
    rigid_body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    space: &Space, 
    texture_path: &PathBuf, 
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

    let draw_pos = rapier_to_macroquad(position.vector);

    draw_texture_ex(
        textures.get(texture_path), 
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

pub fn contains_point(collider_handle: ColliderHandle, space: &mut Space, point: Vector2<f32>) -> bool {
    let mut contains_point: bool = false;

    space.query_pipeline.update(&space.collider_set);

    space.query_pipeline.intersections_with_point(
        &space.rigid_body_set, &space.collider_set, &point![point.x, point.y], QueryFilter::default(), |handle| {
            if collider_handle == handle {
                contains_point = true;
                return false
            }

            return true
        }
    );

    contains_point
} 

pub fn draw_hitbox(space: &Space, rigid_body_handle: RigidBodyHandle, collider_handle: ColliderHandle, color: Color) {
    let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();
    let collider = space.collider_set.get(collider_handle).unwrap();

    let shape = collider.shape().as_cuboid().unwrap();

    let position = collider.position().translation;
    let rotation = rigid_body.rotation().angle();

    let draw_pos = rapier_to_macroquad(position.vector);

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

pub fn rapier_mouse_world_pos(camera_rect: &Rect) -> Vector2<f32> {

    
    let pos = macroquad_to_rapier(
        &mouse_world_pos(camera_rect)
    );

    Vector2::new(pos.x, pos.y)
}

pub fn get_angle_to_mouse(point: Vector2<f32>, camera_rect: &Rect) -> f32 {

    let mouse_pos = rapier_mouse_world_pos(camera_rect);

    let distance_to_mouse = Vec2::new(
        mouse_pos.x - point.x,
        mouse_pos.y - point.y 
    );

    distance_to_mouse.x.atan2(distance_to_mouse.y)
}

pub fn get_angle_between_rapier_points(point_1: Vector2<f32>, point_2: Vector2<f32>) -> f32 {

    let distance_to_mouse = Vec2::new(
        point_2.x - point_1.x,
        point_2.y - point_1.y 
    );

    distance_to_mouse.x.atan2(distance_to_mouse.y)
}

pub struct ClientIO {
    pub send: WsSender,
    pub receive: WsReceiver,
    pub packet_queue: Vec<NetworkPacket>
}


impl ClientIO {
    pub fn send_network_packet(&mut self, packet: NetworkPacket) {

        self.packet_queue.push(packet);
        

    }

    pub fn flush(&mut self) {

        self.send.send(
            ewebsock::WsMessage::Binary(
                bitcode::serialize(&self.packet_queue).unwrap()
            )
        );

        self.packet_queue.clear();
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


                            return Vec::new();
                            // this is stupid
                            // if error.contains("A non-blocking socket operation could not be completed immediately)") {
                            //     println!("io error: {}", error);
                            //     return Vec::new();
                            // }
                            //todo!("unhandled 'Error' event when trying to receive update from server: {}", error)
                        },
                        ewebsock::WsEvent::Closed => todo!("server closed"),
                    }
                },
                None => break, // this means there are no more updates
            };

            let mut network_packets: Vec<NetworkPacket> = bitcode::deserialize(&network_packet_bytes).unwrap();

            packets.append(&mut network_packets);

        }

        packets
    }

}

pub fn draw_preview(textures: &TextureLoader, size: f32, draw_pos: Vec2, color: Option<Color>, rotation: f32, texture_path: &PathBuf) { 
    let color = color.unwrap_or(WHITE);

    let dest_size = get_preview_resolution(size, textures, texture_path);

    let texture = textures.get(&texture_path);

    let mut params = DrawTextureParams::default();

    params.dest_size = Some(
        dest_size
    );

    params.rotation = rotation;


    draw_texture_ex(
        texture, 
        draw_pos.x, 
        draw_pos.y + (size - dest_size.x), // sit on the bottom if that makes sense 
        color,
        params
    );
}
pub fn get_preview_resolution(size: f32, textures: &TextureLoader, texture_path: &PathBuf) -> Vec2 {
    let texture = textures.get(texture_path);

    // size represents the max size in pixels a pixel can be in either width or height BUT we dont want to change the aspect ratio so hence this magic
    return match texture.width() > texture.height() {
        true => {
            // height conforms to width
            let ratio = texture.height() / texture.width();

            Vec2::new(size, size * ratio)

        },
        false => {
            // width conforms to height
            let ratio = texture.width() / texture.height();

            Vec2::new(size * ratio, size)
        },
    };

}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ClientId {
    id: u64
}
impl ClientId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64()
        }
    }
}
pub struct ServerIO {
    pub clients: HashMap<ClientId, WebSocket<TcpStream>>,
    pub listener: TcpListener,
    queued_packets: HashMap<ClientId, Vec<NetworkPacket>>
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
            queued_packets: HashMap::new()
        }
    }

    /// Send the queued packets and return disconnected client ids
    pub fn flush(&mut self, total_sent_bytes: &mut usize) -> Vec<ClientId> {

        let mut disconnected_clients: Vec<ClientId> = Vec::new();

        for (client_id, client) in &mut self.clients {
            let queued_packets = match self.queued_packets.get(client_id) {
                Some(queued_packets) => {

                    if queued_packets.len() == 0 {
                        continue;
                    }

                    queued_packets
                },
                None => panic!("didn't have a packet queue for client: {:?}", client_id),
            };

            let bytes = bitcode::serialize(&queued_packets).unwrap();

            *total_sent_bytes += bytes.len();

            match client.send(
                
                tungstenite::Message::Binary(
                    bytes.into()
                )
            ) {
                Ok(_) => {
                    self.queued_packets.get_mut(client_id).unwrap().clear();
                },
                Err(_) => {
                    disconnected_clients.push(*client_id);
                },
            }


        }

        for client in &disconnected_clients {
            self.clients.remove(&client).unwrap();
            self.queued_packets.remove(&client);
        };

        disconnected_clients
    }

    pub fn send_all_except(&mut self, packet: NetworkPacket, except: ClientId) {

        for client_id in &mut self.clients.keys() {

            if *client_id == except {
                continue;
            }

            let queued_packets = self.queued_packets.get_mut(client_id).unwrap();


            queued_packets.push(packet.clone());
            
        }

        
    }

    pub fn send_all_clients(&mut self, packet: NetworkPacket) {


        for client_id in &mut self.clients.keys() {

            let queued_packets = self.queued_packets.get_mut(client_id).unwrap();

            queued_packets.push(packet.clone());
            
            
        }


    }

    pub fn send_client(&mut self, client_id: ClientId, packet: NetworkPacket) {
        let queued_packets = self.queued_packets.get_mut(&client_id).unwrap();

        queued_packets.push(packet);
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

                self.queued_packets.insert(client_id, Vec::new());

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
    pub network_io: &'a mut ClientIO,
    pub last_tick_duration: &'a web_time::Duration,
    pub client_id: &'a ClientId,
    pub camera_rect: &'a mut Rect,
    pub prefabs: &'a Prefabs,
    pub screen_shake: &'a mut ScreenShakeParameters,
    pub sounds: &'a SoundLoader,
    pub textures: &'a TextureLoader,
    pub camera: &'a Camera2D
}

pub struct Prefabs {
    prefabs: HashMap<String, String>
}

impl Prefabs {

    pub fn new() -> Self {
        Self {
            prefabs: HashMap::new(),
        }
    }
    pub fn get_prefab_data(&self, path: impl ToString) -> String {
        
        self.prefabs.get(&path.to_string()).unwrap().clone()
    }

    pub async fn load_prefab_data(&mut self, path: impl ToString) {

        let data = load_string(&path.to_string()).await.unwrap();

        self.prefabs.insert(path.to_string(), data);
    }

    pub fn load_prefab_data_block(&mut self, path: impl ToString) {

        let data = read_to_string(path.to_string()).unwrap();

        self.prefabs.insert(path.to_string(), data);
    }
}

#[cfg(target_arch = "x86_64")]
pub fn log(message: &str) {
    println!("{message}");
}

#[cfg(target_arch = "wasm32")]
pub fn log(message: &str) {
    web_sys::console::log_1(&message.into());
}

pub fn macroquad_to_rapier(macroquad_coords: &Vec2) -> Vec2 {

    // translate macroquad coords to rapier coords
    Vec2 { 
        x: macroquad_coords.x, 
        y: (macroquad_coords.y * -1.) + 720.
    }
}
