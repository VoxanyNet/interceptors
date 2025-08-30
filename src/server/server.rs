use std::fs::read_to_string;

use interceptors_lib::{area::{Area, AreaId, AreaSave}, bullet_trail::BulletTrail, dropped_item::DroppedItem, enemy::Enemy, player::{ItemSlot, Player}, prop::{Prop, PropUpdateOwner}, updates::{LoadArea, NetworkPacket}, weapon::WeaponType, world::World, ClientId, Prefabs, ServerIO};
use tungstenite::Message;

include!(concat!(env!("OUT_DIR"), "/prefabs.rs"));

pub struct Server {
    world: World,
    prefabs: Prefabs,
    last_tick: web_time::Instant,
    last_tick_duration: web_time::Duration,
    network_io: ServerIO,
    total_bits_sent: usize,
    previous_tick_connected_clients: Vec<ClientId>
}

impl Server {
    pub fn new() -> Self {

        let mut world = World::empty();

        let lobby_save: AreaSave = serde_json::from_str(&read_to_string("areas/ship.json").unwrap()).unwrap();
        
        let mut prefabs = Prefabs::new();

        for prefab_path in PREFAB_PATHS {
            prefabs.load_prefab_data_block(prefab_path)
        }

        world.areas.push(Area::from_save(lobby_save, None, &prefabs));

        


        Self {
            last_tick: web_time::Instant::now(),
            last_tick_duration: web_time::Duration::from_micros(1),
            network_io: ServerIO::new(),
            world,
            total_bits_sent: 0,
            previous_tick_connected_clients: Vec::new(),
            prefabs
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

    pub fn get_connected_clients_vector(&self) -> Vec<ClientId> {

        let mut connected_clients = Vec::new();

        for client_id in self.network_io.clients.keys() {
            connected_clients.push(*client_id);
        };

        connected_clients
    }

    pub fn handle_disconnected_client(&mut self, client_id: ClientId) {
        if self.network_io.clients.keys().len() == 0 {

            let lobby: AreaSave = serde_json::from_str(&read_to_string("areas/ship.json").unwrap()).unwrap();
            self.world.areas[0] = Area::from_save(lobby, Some(AreaId::new()), &self.prefabs)
        }
    }

    pub fn run(&mut self) {
        loop {

            let mut disconnected_clients = Vec::new();

            for client_id in &self.previous_tick_connected_clients {
                if !self.get_connected_clients_vector().contains(&client_id) {
                    disconnected_clients.push(client_id.clone());
                }
            }

            for client in disconnected_clients {
                self.handle_disconnected_client(client);
            }


            self.previous_tick_connected_clients = self.get_connected_clients_vector();

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

                    let mut prop = area.props.iter_mut().find(|prop| {prop.id == update.id});

                    if prop.is_none() {
                        if let Some(computer) = &mut area.computer {
                            if computer.prop.id == update.id {
                                prop = Some(&mut computer.prop);
                            }
                        }
                    }

                    if let Some(prop) = prop {
                        prop.set_velocity(update.velocity, &mut area.space);

                        self.network_io.send_all_except(network_packet, client_id);
                    }
                    


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

                NetworkPacket::NewDroppedItemUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    area.dropped_items.push(
                        DroppedItem::from_save(update.dropped_item.clone(), &mut area.space, &self.prefabs)
                    );

                    self.network_io.send_all_except(network_packet, client_id);
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

                    self.network_io.send_all_except(network_packet, client_id);
                }
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
                NetworkPacket::PlayerHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.health = update.health;

                    self.network_io.send_all_except(network_packet, client_id);
                }
                NetworkPacket::DroppedItemVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let dropped_item = area.dropped_items.iter_mut().find(|dropped_item| {dropped_item.id == update.id}).unwrap();

                    dropped_item.set_velocity(&mut area.space, update.velocity);

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
                },
                NetworkPacket::PlayerFacingUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.id}).unwrap();

                    player.set_facing(update.facing);

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::SpawnBulletTrail(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    area.bullet_trails.push(
                        BulletTrail::from_save(update.save)
                    );

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::PlayerPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    let current_pos = area.space.rigid_body_set.get(player.body.body_handle).unwrap().position();

                    player.set_pos(update.pos, &mut area.space);

                    self.network_io.send_all_except(network_packet, client_id);

                
                },
                NetworkPacket::PropPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = match area.props.iter_mut().find(|prop| {prop.id} == update.prop_id) {
                        Some(prop) => prop,
                        None => {
                            println!("received bad update for prop");
                            continue;
                        },
                    };

                    let current_pos = area.space.rigid_body_set.get(prop.rigid_body_handle).unwrap().position();

                    if (update.pos.translation.x - current_pos.translation.x).abs() > 20. {
                        prop.set_pos(update.pos, &mut area.space);
                    }

                    self.network_io.send_all_except(network_packet, client_id);

                },
                NetworkPacket::LoadArea(_update) => {
                    panic!("server received client bound load area update");
                },
                NetworkPacket::PropUpdateOwner(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = area.props.iter_mut().find(|prop| {prop.id} == update.id).unwrap();

                    prop.owner = update.owner;

                    self.network_io.send_all_except(network_packet, client_id);
                },

                NetworkPacket::DissolveProp(_update) => {
                    
                    // we can just pass this along to the other clients because the server doesnt really care about the physics of the dissolved props :)))

                    self.network_io.send_all_except(network_packet, client_id);
                }
                NetworkPacket::RemovePropUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let prop = area.props.iter_mut().find(|prop|{prop.id == update.prop_id}).unwrap();

                    prop.despawn(&mut area.space, area.id, None);

                    area.props.retain(|prop|{prop.id != update.prop_id});

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::ActiveItemSlotUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.selected_item = update.active_item_slot as usize;

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::ItemSlotQuantityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(|area| {area.id == update.area_id}).unwrap();

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

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::ItemSlotUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.inventory.items[update.inventory_index] = match &update.item_slot {
                        Some(item_slot_save) => {
                            Some(
                                ItemSlot::from_save(item_slot_save.clone(), &mut area.space)
                            )
                        },
                        None => None,
                    };

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::ActiveWeaponUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let player = area.players.iter_mut().find(|player| {player.id == update.player_id}).unwrap();

                    player.weapon = match &update.weapon {
                        Some(weapon) => {
                            Some(WeaponType::from_save(&mut area.space, weapon.clone(), Some(player.body.body_handle)))
                        },
                        None => None,
                    };
                    
                    self.network_io.send_all_except(network_packet, client_id);
                },

                NetworkPacket::NewEnemyUpdate(update) => {

                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();


                    let enemy = Enemy::from_save(update.enemy.clone(), &mut area.space);

                    dbg!(enemy.id);

                    area.enemies.push(enemy);

                    self.network_io.send_all_except(network_packet, client_id);
                }
                NetworkPacket::EnemyPositionUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();


                    let enemy = area.enemies.iter().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    area.space.rigid_body_set.get_mut(enemy.body.body_handle).unwrap().set_position(update.position, true);

                    self.network_io.send_all_except(network_packet, client_id);

                },
                NetworkPacket::EnemyVelocityUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    dbg!(update.enemy_id);

                    let enemy = area.enemies.iter().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    area.space.rigid_body_set.get_mut(enemy.body.body_handle).unwrap().set_vels(update.velocity, true);

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::EnemyWeaponUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();


                    enemy.weapon = Some(
                        WeaponType::from_save(&mut area.space, update.weapon.clone(), Some(enemy.body.body_handle))
                    );

                    self.network_io.send_all_except(network_packet, client_id);

                },
                NetworkPacket::EnemyHealthUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    enemy.health = update.health;

                    self.network_io.send_all_except(network_packet, client_id);
                },
                NetworkPacket::EnemyDespawnUpdate(update) => {
                    let area = self.world.areas.iter_mut().find(
                        |area| {
                            area.id == update.area_id
                        }
                    ).unwrap();

                    let enemy = area.enemies.iter_mut().find(|enemy| {enemy.id == update.enemy_id}).unwrap();

                    enemy.despawn(&mut area.space);

                    area.enemies.retain(|enemy|{enemy.id != update.enemy_id});  

                    self.network_io.send_all_except(network_packet, client_id);


                },
                
                
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