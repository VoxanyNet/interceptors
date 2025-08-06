use std::{net::{TcpListener, TcpStream}, time::{Duration, Instant}};

use interceptors_lib::{space::Space, updates::{NetworkPacket, Ping}, ClientId, ServerIO};
use tungstenite::{Message, WebSocket};

pub struct Server {
    space: Space,
    last_tick: Instant,
    last_tick_duration: Duration,
    network_io: ServerIO
}

impl Server {
    pub fn new() -> Self {

        Self {
            space: Space::default(),
            last_tick: Instant::now(),
            last_tick_duration: Duration::from_micros(1),
            network_io: ServerIO::new()
        }

    }

    pub fn run(&mut self) {
        loop {
            self.tick();

            self.network_io.accept_new_client();

            let packets = self.receive_packets();
            self.handle_packets(packets);
        }
    }

    pub fn handle_packets(
        &mut self, 
        network_packets: Vec<(ClientId, NetworkPacket)>, 
    ) {

        for (client_id, network_packet) in network_packets {
            match network_packet {
            NetworkPacket::Ping(ping) => {

                let client = self.network_io.clients.get_mut(&client_id).unwrap();

                // just reply to the ping
                client.send(
                    Message::Binary(
                        bitcode::serialize(
                            &NetworkPacket::Ping(Ping::new_with_id(ping.id))
                        ).unwrap().into()
                    )
                ).unwrap();
            },
        }
        }
        
    }

    pub fn receive_packets(&mut self) -> Vec<(ClientId, NetworkPacket)>{

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

                let packet: NetworkPacket = bitcode::deserialize(&update_bytes).unwrap();

                packets.push((*client_id, packet));
                
            }
        }

        for disconnected_client in disconnected_clients {
            self.network_io.clients.remove(&disconnected_client);
        }
        packets
    }

    pub fn tick(&mut self) {
        self.space.step(self.last_tick_duration);

        self.last_tick_duration = self.last_tick.elapsed();
        self.last_tick = Instant::now();
    }
}