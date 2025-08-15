use std::fs::read_to_string;

use interceptors_lib::{area::{Area, AreaSave}, player::Player, prop::{Prop, PropUpdateOwner}, updates::{LoadArea, NetworkPacket}, world::World, ClientId, ServerIO};
use tungstenite::Message;

pub struct Server {
    world: World,
    last_tick: web_time::Instant,
    last_tick_duration: web_time::Duration,
    network_io: ServerIO,
    total_bits_sent: usize
}

impl Server {
    pub fn new() -> Self {

        let mut world = World::empty();

        let lobby_save: AreaSave = serde_json::from_str(&read_to_string("areas/lobby.json").unwrap()).unwrap();

        world.areas.push(Area::from_save(lobby_save, None));

        Self {
            last_tick: web_time::Instant::now(),
            last_tick_duration: web_time::Duration::from_micros(1),
            network_io: ServerIO::new(),
            world,
            total_bits_sent: 0
        }

    }

    pub fn handle_new_client(&mut self, new_client: ClientId) {


        self.network_io.send_client(new_client, NetworkPacket::LoadArea(

            // TEMPORARILY JUST SENDING THE FIRST AREA BUT WE NEED A WAY TO DESIGNATE A SPAWN AREA
            LoadArea {
                area: self.world.areas[0].save(),
                id: self.world.areas[0].id
            }
        ));

        // if this is the first client we give them ownership of all the props
        if self.network_io.clients.len() == 1 {
            for area in &mut self.world.areas {
                for prop in &mut area.props {
                    self.network_io.send_all_clients(NetworkPacket::PropUpdateOwner(PropUpdateOwner{ owner: Some(new_client), id: prop.id, area_id: area.id }));
                }
            }
        }
    }

    pub fn run(&mut self) {
        loop {

            // only tick every 8 ms
            if self.last_tick.elapsed().as_millis() > 8 {
                self.tick();
            }
            

            let new_client = self.network_io.accept_new_client();

            if let Some(new_client) = new_client {
                self.handle_new_client(new_client);
            }

            let packets = self.receive_packets();
            self.handle_packets(packets);
        }
    }

    pub fn handle_packets(
        &mut self, 
        network_packets: Vec<(ClientId, NetworkPacket)>, 
    ) {

        for (client_id, network_packet) in network_packets {
            match &network_packet {
                NetworkPacket::Ping(ping) => {

                    let client = self.network_io.clients.get_mut(&client_id).unwrap();


                    self.network_io.send_client(client_id, network_packet.clone());

                },
                NetworkPacket::PropVelocityUpdate(update) => {

                    
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let prop = area.props.iter_mut().find(|prop| {prop.id == update.id}).unwrap();

                    prop.set_velocity(update.velocity, &mut area.space);

                    self.network_io.send_all_except(network_packet, client_id);


                },
                NetworkPacket::NewProp(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.props.push(Prop::from_save(update.prop.clone(), &mut area.space));

                    self.network_io.send_all_except(network_packet, client_id);

                },
                NetworkPacket::NewPlayer(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.players.push(Player::from_save(update.player.clone(), &mut area.space));

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::PlayerVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    player.set_velocity(update.velocity, &mut area.space);

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::PlayerCursorUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    player.set_cursor_pos(update.pos);

                    self.network_io.send_all_except(network_packet, client_id);
                }
                _ => {}
        }
        }
        
    }

    pub fn receive_packets(&mut self) -> Vec<(ClientId, NetworkPacket)>{
        // we should really just return HashMap<ClientId, Vec<NetworkPacket>> but i dont feel like rewriting the handle packets function

        let mut disconnected_clients: Vec<ClientId> = Vec::default();
        let mut packets: Vec<(ClientId, NetworkPacket)> = Vec::new();

        'client_loop: for (client_id, client) in &mut self.network_io.clients  {

            loop {
                let update_bytes = match client.read() {
                    Ok(message) => {
                        match message {
                            Message::Binary(update_bytes) => {
                                update_bytes
                            },
                            Message::Close(_close_message) => {
                                println!("client {:?} disconnected", client_id);

                                disconnected_clients.push(*client_id);

                                continue 'client_loop;
                            },
                            _ => {
                                println!("client tried to send non binary message. disconnecting them!");

                                disconnected_clients.push(*client_id);

                                continue 'client_loop;
                            }
                        }
                    },
                    Err(error) => {
                        match error {

                            tungstenite::Error::Io(io_error) => {
                                match io_error.kind() {
                                    std::io::ErrorKind::WouldBlock => {
                                        // this means that there was no update to read
                                        
                                        continue 'client_loop // move to the next client
                                    },
                                    std::io::ErrorKind::ConnectionReset => {
                                        println!("client {:?} disconnected", client_id);

                                        disconnected_clients.push(*client_id);

                                        continue 'client_loop;
                                    }
                                    _ => todo!("unhandled io error: {}", io_error),
                                }
                            },
                            
                            tungstenite::Error::Protocol(_error) => {
                                println!("client {:?} disconnected due to protocol error", client_id);

                                disconnected_clients.push(*client_id);

                                continue 'client_loop;
                            },
                            
                            _ => todo!("unhandled websocket message read error: {}", error.to_string())
                        }
                    },
                };

                let client_packets: Vec<NetworkPacket> = bitcode::deserialize(&update_bytes).unwrap();

                // we should really just return HashMap<ClientId, Vec<NetworkPacket>>
                for packet in client_packets {
                    packets.push((*client_id, packet));
                }
                
            }
        }

        for disconnected_client in disconnected_clients {
            self.network_io.clients.remove(&disconnected_client);
        }
        packets
    }

    pub fn tick(&mut self) {

        let megabits = self.total_bits_sent as f32 / 1000000 as f32;

        //dbg!(megabits);

        

        self.world.server_tick(&mut self.network_io, self.last_tick_duration);

        self.network_io.flush(&mut self.total_bits_sent);

        self.last_tick_duration = self.last_tick.elapsed();
        self.last_tick = web_time::Instant::now();
    }
}