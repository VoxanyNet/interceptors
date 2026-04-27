use std::{collections::VecDeque, path::PathBuf, str::FromStr};

use glamx::{Pose2, Vec2, vec2};
use macroquad::{camera::Camera2D, color::{RED, WHITE}, input::{KeyCode, is_key_released}, math::Rect, prelude::{gl_use_default_material, gl_use_material}, shapes::{draw_circle, draw_rectangle}, time::get_time, window::{clear_background, screen_height, screen_width}};
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize, de};

use crate::{
    ClientId, ClientTickContext, Owner, Prefabs, ServerIO, SwapIter, TextureLoader, TickContext, ambiance::{Ambiance, AmbianceSave}, background::{self, Background, BackgroundSave}, base_prop::{BaseProp, NewProp, PropId}, base_prop_save::BasePropSave, bullet_trail::BulletTrail, clip::{Clip, ClipSave}, compound_test::CompoundTest, computer::Computer, decoration::{Decoration, DecorationSave}, dissolved_pixel::DissolvedPixel, drawable::{DrawContext, Drawable}, dropped_item::{DroppedItem, DroppedItemSave}, enemy::{Enemy, EnemySave, NewEnemyUpdate}, font_loader::FontLoader, material_loader::MaterialLoader, player::{Facing, NewPlayer, Player, PlayerSave}, prop::Prop, prop_save::PropSave, rapier_mouse_world_pos, rapier_to_macroquad, selectable_object_id::{SelectableObject, SelectableObjectId}, sound_loader::SoundLoader, space::Space, texture_loader::ClientTextureLoader, tile::{Tile, TileSave}, updates::NetworkPacket, uuid_u64, weapons::{bullet_impact_data::BulletImpactData, smg::weapon::SMG, weapon::weapon::WeaponOwner}};

macro_rules! test {
    ($s:ident) => {
        println!("hello world!")
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct AreaId {
    id: u64
}

impl AreaId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64()
        }
    }
}
pub struct Area {
    pub backgrounds: Vec<Background>,
    pub spawn_point: Vec2,
    pub space: Space,
    pub decorations: Vec<Decoration>,
    pub clips: Vec<Clip>,
    pub players: Vec<Player>,
    pub props: Vec<Box<dyn Prop>>,
    pub id: AreaId,
    pub bullet_trails: Vec<BulletTrail>,
    pub dissolved_pixels: Vec<DissolvedPixel>,
    pub enemies: Vec<Enemy>,
    pub computer: Option<Computer>,
    pub dropped_items: Vec<DroppedItem>,
    pub max_camera_y: f32,
    pub minimum_camera_width: f32,
    pub minimum_camera_height: f32,
    pub despawn_y: f32,
    pub master: Option<ClientId>,
    pub ambiance: Vec<Ambiance>,
    pub wave_data: WaveData,
    pub compound_test: Vec<CompoundTest>,
    pub tiles: Vec<Vec<Option<Tile>>>,
    pub impact_points: Vec<glamx::Vec2>,
    pub bullet_impact_queue: Vec<BulletImpactData>
}

pub struct WaveData {
    wave: i32,
    wave_end: web_time::Instant,
    wave_start: web_time::Instant,
    last_batch_spawn: web_time::Instant,
    batch_size: u32,
    total_size: u32,
    spawned_this_wave: u32,
    batch_interval: web_time::Duration,
    active: bool
}

impl WaveData {
    pub fn default() -> Self {
        WaveData {
            wave: 1,
            wave_end: web_time::Instant::now(),
            wave_start: web_time::Instant::now(),
            last_batch_spawn: web_time::Instant::now(),
            batch_size: 2,
            total_size: 10,
            spawned_this_wave: 0,
            batch_interval: web_time::Duration::from_secs_f32(5.),
            active: false

        }
    }
}

impl Area { 
    
    pub fn tick(&mut self, ctx: &mut TickContext) {



        self.space.step(ctx.last_tick_duration());

        if let TickContext::Client(ctx) = ctx {
            self.start_ambiance(ctx.sounds);
            self.spawn_player_if_not_in_game(ctx);
            self.debug_spawn_prop(ctx);
            self.debug_spawn_enemy(ctx);
        };

        self.handle_bullet_impacts(ctx);
        self.tick_entities(ctx);
        self.despawn_entities(ctx);
    }



    pub fn tick_entities(&mut self, ctx: &mut TickContext) {
        self.tick_enemies(ctx);
        self.tick_props(ctx);
        self.tick_dissolved_pixels();
        self.tick_bullet_trails(ctx);   
        self.tick_players(ctx); 

        // might want to make this server + client side eventually
        if let TickContext::Client(ctx) = ctx {
            
            self.tick_computer(ctx);
        }
        
    }

    
    pub fn draw(
        &mut self, 
        ctx: &mut TickContext
    ) {

        for decoration in &self.decorations {
            decoration.draw(ctx)
        }


        for prop in &mut self.props {
            prop.draw(ctx, &mut self.space)
        }

        for background in &mut self.backgrounds {
            background.draw(ctx);
        }

        for dropped_item in &self.dropped_items {
            dropped_item.draw(ctx, &mut self.space);
        }

        if let Some(computer) = &mut self.computer {
            computer.draw(ctx, &mut self.space);
        }

        for player in &mut self.players {
            player.draw(ctx, &self.space);
        }

        for enemy in &mut self.enemies {
            enemy.draw(ctx, &mut self.space);
        }

        for clip in &self.clips {
            clip.draw(ctx, &self.space)
        }

    
        //self.computer.draw()

        // backgrounds are handled seperately because they are always drawn below everything and dont have a layer

        // let material = draw_context.materials.get("materials/stars");
        // material.set_uniform("Time", get_time() as f32);
        // material.set_uniform("Resolution", vec2(500., 500.));

        // gl_use_material(material);
        // draw_rectangle(0., 0., 5000., 5000., WHITE);
        // gl_use_default_material();


        for impact_point in &self.impact_points {
            let m_pos = rapier_to_macroquad(*impact_point);
            let mut color = RED;
            color.a = 0.2;
            draw_circle(m_pos.x, m_pos.y, 5., color);
        }



    }

    pub fn draw_hud(&self, textures: &ClientTextureLoader) {
        for player in &self.players {
            player.draw_hud(textures);
        }
    }

    pub fn tick_enemies(&mut self, ctx: &mut TickContext) {

        //let mut wasted_time = Duration::ZERO;

        //let enemies_vec_ptr = &mut self.enemies as *mut Vec<Enemy>;
        
        //let then = Instant::now();
        let mut enemy_iter = SwapIter::new(&mut self.enemies);
        //wasted_time += then.elapsed();

        while enemy_iter.not_done() {
            // let then = Instant::now();
            let (enemies, mut enemy) = enemy_iter.next();

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: &mut self.players,
                props: &mut self.props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue,
            };

            enemy.tick(
                ctx,
                &mut area_context
            );

            enemy_iter.restore(enemy);

        }

    }

    pub fn tick_props(&mut self, ctx: &mut TickContext) {

        let mut props_iter = SwapIter::new(&mut self.props);

        while props_iter.not_done() {

            let (props, mut prop) = props_iter.next();

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: &mut self.players,
                props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies: &mut self.enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue,
            };

            prop.tick(&mut area_context, ctx);

            props_iter.restore(prop);;;;;; // i loveeee semicolons
            
        }
    }

    pub fn tick_dissolved_pixels(&mut self) {
        for dissolved_pixel in &mut self.dissolved_pixels {
            dissolved_pixel.tick();
        }
    }

    pub fn tick_bullet_trails(&mut self, ctx: &mut TickContext) {
        for bullet_trail in &mut self.bullet_trails {
            bullet_trail.tick(ctx);
        }
    } 
    
    pub fn tick_players(&mut self, ctx: &mut TickContext) {

        let mut players_iter = SwapIter::new(&mut self.players);
        
        while players_iter.not_done() {
            let (players, mut player) = players_iter.next();

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: players,
                props: &mut self.props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies: &mut self.enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue
            };
            player.client_tick(
                ctx, 
                &mut area_context
            );

            players_iter.restore(player);
        }
    }

    pub fn tick_computer(&mut self, ctx: &mut ClientTickContext) {
        if let Some(computer) = &mut self.computer {
            computer.tick(ctx, &mut self.players, &self.space);
        }
    }


    pub fn get_selectable_object_mut(&mut self, selectable_object_id: SelectableObjectId) -> Option<SelectableObject<'_>> {
        match selectable_object_id {
            SelectableObjectId::Decoration(decoration_index) => {
                if let Some(decoration) = self.decorations.get_mut(decoration_index) {
                    Some(
                        SelectableObject::Decoration(decoration)
                    )
                } else {
                    None
                }
            },
            SelectableObjectId::Tile(_location) => {
                None
            },
            SelectableObjectId::Prop(prop_id) => {
                if let Some(prop) = self.props.iter_mut().find(|prop| {prop.id() == prop_id}) {
                    Some(SelectableObject::Prop(prop))
                } else {
                    None
                }
            },
            SelectableObjectId::Clip(clip_index) => {
                if let Some(clip) = self.clips.get_mut(clip_index) {
                    Some(
                        SelectableObject::Clip(clip)
                    )
                } else {
                    None
                }
            },
                    }
    }

    pub fn get_tile_at_position_mut(&mut self, tile_pos: Vec2) -> Option<&mut Tile> {
        let tile_index_x = (tile_pos.x / 50.) as usize;
        let tile_index_y = (tile_pos.y / 50.) as usize;

        self.get_tile_index((tile_index_x, tile_index_y))
    }

    pub fn get_tile_index(&mut self, index: (usize, usize)) -> Option<&mut Tile> {
        if let Some(column) =  self.tiles.get_mut(index.0) {
            if let Some(tile) = column.get_mut(index.1) {
                return tile.as_mut()
            } else {
                return  None;
            }
        } else {
            return  None;
        }
    }
    pub fn generate_terrain(&mut self, seed: u32) {

        let perlin = Perlin::new(seed); // seed = 0
        let world_width = 256;
        let world_height = 64;

        let mut terrain: Vec<Vec<bool>> = vec![vec![false; world_height]; world_width];

        let scale_factor = 50.;

        let height_multiplier = 0.1;

        for x in 0..world_width {
            let noise_val = perlin.get([x as f64 / scale_factor, 0.0]);
            let height = ((noise_val + 1.0) / 2.0 * world_height as f64 * height_multiplier) as usize + 20;

            for y in 0..world_height {
                if y <= height {
                    terrain[x][y] = true;
                } else {
                    terrain[x][y] = false; 
                }
            }
        }

        for x in 0..world_width {
            
            for y in 0..world_height {

                if terrain[x][y] {

                    self.tiles[x].insert(y, Some(Tile::new(PathBuf::from_str("assets/dirt.png").unwrap())));
                    
                }
            }
        }



    }
    pub fn empty() -> Self {

        let world_height = 500;
        let world_width = 500;
    
        Self {
            spawn_point: Vec2::ZERO,
            space: Space::new(),
            decorations: Vec::new(),
            clips: Vec::new(),
            players: Vec::new(),
            backgrounds: Vec::new(),
            props: Vec::new(),
            id: AreaId::new(),
            bullet_trails: Vec::new(),
            dissolved_pixels: Vec::new(),
            enemies: Vec::new(),
            computer: None,
            dropped_items: Vec::new(),
            max_camera_y: 0.,
            minimum_camera_width: 1920.,
            minimum_camera_height: 1080.,
            despawn_y: 0.,
            master: None,
            ambiance: Vec::new(),
            wave_data: WaveData::default(),
            compound_test: Vec::new(),
            tiles: vec![vec![None; world_height]; world_width],
            impact_points: vec![],
            bullet_impact_queue: Vec::new()
        }
    }

    // pub fn get_drawable_objects_self(&self) -> Vec<&dyn Drawable> {
    //     // there are some situations in which we need to maintain ownership of self so thats why these methods are split
    //     Self::get_drawable_objects(
    //         &self.backgrounds, 
    //         &self.decorations, 
    //         &self.props, 
    //         &self.dropped_items, 
    //         &self.computer, 
    //         &self.players, 
    //         &self.enemies, 
    //         &self.dissolved_pixels, 
    //         &self.bullet_trails,
    //         &self.clips
    //     )
    // }
    // pub fn get_drawable_objects<'a> (
    //     backgrounds: &'a Vec<Background>,
    //     decorations: &'a Vec<Decoration>,
    //     props: &'a Vec<Box<dyn Prop>>,
    //     dropped_items: &'a Vec<DroppedItem>,
    //     computer: &'a Option<Computer>,
    //     players: &'a Vec<Player>,
    //     enemies: &'a Vec<Enemy>,
    //     dissolved_pixels: &'a Vec<DissolvedPixel>,
    //     bullet_trails: &'a Vec<BulletTrail>,
    //     clips: &'a Vec<Clip>
    // ) -> Vec<&'a dyn Drawable> {
    //     let mut drawable_objects: Vec<&dyn Drawable> = vec![];
        
    //     for background in backgrounds {
    //         drawable_objects.push(background);
    //     }
    //     for decoration in decorations {
    //         drawable_objects.push(decoration);
    //     }
    //     for prop in props {
    //         drawable_objects.push(prop.as_ref() as &dyn Drawable); // need to learn why this works
    //     }
    //     // for dropped_item in dropped_items {
    //     //     drawable_objects.push(dropped_item);
    //     // }
    //     // if let Some(computer) = computer {
    //     //     drawable_objects.push(computer);
    //     // }
    //     // for player in players {
    //     //     drawable_objects.push(player);
    //     // }
    //     // for enemy in enemies {
    //     //     drawable_objects.push(enemy);
    //     // }
    //     for pixel in dissolved_pixels {
    //         drawable_objects.push(pixel);
    //     }
    //     for bullet_trail in bullet_trails {
    //         drawable_objects.push(bullet_trail);
    //     }
    //     for clip in clips {
    //         drawable_objects.push(clip);
    //     }

    //     drawable_objects
    // }
    // pub fn get_drawable_objects_mut<'a> (
    //     decorations: &'a mut Vec<Decoration>,
    //     props: &'a mut Vec<Box<dyn Prop>>,
    //     dropped_items: &'a mut Vec<DroppedItem>,
    //     computer: &'a mut Option<Computer>,
    //     players: &'a mut Vec<Player>,
    //     enemies: &'a mut Vec<Enemy>,
    //     dissolved_pixels: &'a mut Vec<DissolvedPixel>,
    //     bullet_trails: &'a mut Vec<BulletTrail>,
    //     clips: &'a mut Vec<Clip>
    // ) -> Vec<&'a mut dyn Drawable> {
    //     let mut drawable_objects: Vec<&mut dyn Drawable> = vec![];
        
    //     for decoration in decorations {
    //         drawable_objects.push(decoration);
    //     }
    //     for prop in props {
    //         drawable_objects.push(prop.as_mut() as &mut dyn Drawable);
    //     }
    //     // for dropped_item in dropped_items {
    //     //     drawable_objects.push(dropped_item);
    //     // }
    //     // if let Some(computer) = computer {
    //     //     drawable_objects.push(computer);
    //     // }
    //     // for player in players {
    //     //     drawable_objects.push(player);
            
            
    //     // }
    //     // for enemy in enemies {
    //     //     drawable_objects.push(enemy);
    //     // }
    //     for pixel in dissolved_pixels {
    //         drawable_objects.push(pixel);
    //     }
    //     for bullet_trail in bullet_trails {
    //         drawable_objects.push(bullet_trail);
    //     }
    //     for clip in clips {
    //         drawable_objects.push(clip);
    //     }

    //     drawable_objects
    // }


    // pub fn designate_master(&mut self, server_io: &mut ServerIO) {
    //     if self.master == None {
    //         if let Some(player) = self.players.get(0) {
    //             self.master = Some(player.owner);

    //             server_io.send_all_clients(
    //                 NetworkPacket::MasterUpdate(
    //                     MasterUpdate {
    //                         area_id: self.id,
    //                         master: self.master.unwrap().clone(),
    //                     }
    //                 ), 
    //             );
    //         }
    //     }
    // }

    pub fn spawn_player(&mut self, ctx: &mut ClientTickContext) {

        let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

        let player = Player::new(
            Pose2::new(
                glamx::vec2(mouse_pos.x, mouse_pos.y),
                0.
            ), 
            &mut self.space, 
            Owner::ClientId(*ctx.client_id)
        );

        ctx.network_io.send_network_packet(
            NetworkPacket::NewPlayer(
                NewPlayer {
                    player: player.save(&mut self.space),
                    area_id: self.id,
                }
            )
        );

        self.players.push(player);
    }

    pub fn debug_spawn_prop(&mut self, ctx: &mut ClientTickContext) {

        if is_key_released(KeyCode::E) {
            
            let prefab_save: BasePropSave = serde_json::from_str(&ctx.prefabs.get_prefab_data("prefabs\\generic_physics_props\\box2.json")).unwrap();

            let mut new_prop = BaseProp::from_save(prefab_save, &mut self.space, ctx.textures.into());

            new_prop.owner = Some(Owner::ClientId(*ctx.client_id));

            let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

            new_prop.set_pos(
                Pose2::new(
                    vec2(mouse_pos.x, mouse_pos.y),
                    0.
                ), 
                &mut self.space
            );


            ctx.network_io.send_network_packet(
                NetworkPacket::NewProp(
                    NewProp
                    {
                        prop: new_prop.inner_save(&mut self.space).into(), 
                        area_id: self.id
                    }
                )
            );

            self.props.push(Box::new(new_prop));
        }
    }

    pub fn debug_spawn_enemy(&mut self, ctx: &mut ClientTickContext) {

        if !is_key_released(KeyCode::T) {
            return;
        }
        let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);
        
        let enemy = Enemy::new( 
            Pose2::new(
                mouse_pos,
                0.
            ), 
            crate::Owner::Server, 
            &mut self.space, 
            None
        );

        ctx.network_io.send_network_packet(crate::updates::NetworkPacket::NewEnemyUpdate(
            NewEnemyUpdate {
                area_id: self.id,
                enemy: enemy.save(&mut self.space),
            }
        ));

        self.enemies.push(enemy);
    }

    // the player should be spawned by the server - this is temporary
    pub fn spawn_player_if_not_in_game(
        &mut self, ctx: &mut ClientTickContext
    ) {

        match self.players.iter().find(|player| player.owner == Owner::ClientId(*ctx.client_id)) {
            Some(_) => return,
            None => {

                let mut player = Player::new(
                    Pose2::new(
                        self.spawn_point,
                        0.
                    ), 
                    &mut self.space, 
                    Owner::ClientId(*ctx.client_id)
                );
            
                ctx.network_io.send_network_packet(
                    NetworkPacket::NewPlayer(
                        NewPlayer {
                            player: player.save(&self.space),
                            area_id: self.id,
                        }
                    )
                );
                
                player.inventory.try_insert_into_inventory(
                    Box::new(
                        SMG::new(
                            WeaponOwner::Player(player.id), 
                            Some(player.body.body_handle), 
                            Facing::Left
                        )
                    ), 
                    ctx, 
                    self.id, 
                    &mut self.space, 
                    player.id
                );


                self.players.push(
                    player
                );

                

                

                
            },
        }
    }

    pub fn start_ambiance(&mut self, sounds: &mut SoundLoader) {
        
        for ambiance in &mut self.ambiance {
            ambiance.start_if_stopped(sounds);


        }
    }

    pub fn wave_logic(&mut self, ctx: &mut ClientTickContext) {

        // end wave
        if self.wave_data.spawned_this_wave >= self.wave_data.total_size {
            self.wave_data.wave_end = web_time::Instant::now();

            self.wave_data.spawned_this_wave = 0;

            self.wave_data.active = false;

            
        }

        // start new wave
        if self.wave_data.wave_end.elapsed().as_secs_f32() > 5. && self.wave_data.active == false  {

            self.wave_data.wave += 1;

            self.wave_data.batch_size += 2;

            self.wave_data.total_size += 10;

            self.wave_data.active = true;

            self.wave_data.wave_start = web_time::Instant::now();
        }

        // spawn batch
        if self.wave_data.active && self.wave_data.last_batch_spawn.elapsed() > self.wave_data.batch_interval && self.enemies.len() == 0 {

            for i in 0..self.wave_data.batch_size {

                let enemy = Enemy::new(
                    Pose2::new(
                        glamx::vec2(2400. + (i as f32 * 50.), 200.),
                        0.
                    ), 
                    crate::Owner::Server, 
                    &mut self.space, 
                    None
                
                );

                ctx.network_io.send_network_packet(
                    NetworkPacket::NewEnemyUpdate(
                        NewEnemyUpdate {
                            area_id: self.id,
                            enemy: enemy.save(&mut self.space),
                        }
                    )
                );
                self.enemies.push(
                    enemy
                );
            }

            self.wave_data.last_batch_spawn = web_time::Instant::now();
          
        }
    }

    pub fn despawn_entities(&mut self, ctx: &mut TickContext) {
        self.dropped_items.retain_mut(
            |dropped_item|
            {
                if !dropped_item.despawn {
                    return true;
                }

                dropped_item.despawn_callback(&mut self.space);

                false
            }
        );

    
        self.props.retain_mut(
            |prop|
            {
                if !prop.should_despawn() {
                    return true;
                }

                prop.despawn_callback(&mut self.space);

                false
            }
        );
        self.enemies.retain_mut(
            |enemy| 
            {
                if !enemy.despawn {
                    return true;
                }

                enemy.despawn_callback(&mut self.space);
                false
            }
        );
        self.dissolved_pixels.retain_mut(
            |pixel| 
            {

                if !pixel.despawn {
                    return true;
                }
                
                pixel.despawn_callback(&mut self.space);
                false
            }
        );
        self.decorations.retain_mut(
            |decoration| 
            {
                if !decoration.despawn {
                    return true;
                }
                
                decoration.despawn_callback();
                false
            }
        );
        self.clips.retain_mut(
            |clip| 
            {

                if !clip.despawn {
                    return true;
                }

                clip.despawn_callback(&mut self.space);
                false
            }
        );

        let mut players_iter = SwapIter::new(&mut self.players);

        while players_iter.not_done() {
            let (players, mut player) = players_iter.next();

            if !player.despawn {
                players_iter.restore(player);

                continue;
            }

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: players,
                props: &mut self.props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies: &mut self.enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue,
            };

            player.despawn_callback(ctx, &mut area_context);
            
        }
        
    }

    pub fn handle_bullet_impacts(
        &mut self,
        ctx: &mut TickContext
    ) {

        // i'm doing this because i'm lazy but it shouldnt have any effects
        // basically i can't borrow bullet impact queue for the context while iterating over it
        // could use a swapiter but i dont feel like it
        // this gets the job done and callbacks can still add bullet impacts 
        // i like writing comments because rust analyzer cant yell at me here
        let bullet_impact_queue = self.bullet_impact_queue.clone();
        self.bullet_impact_queue.clear();

        // PLAYERS
        for player in &mut self.players {

            let body_collider = player.body.collider_handle;
            let head_collider = player.head.collider_handle;

            for impact in self.bullet_impact_queue.iter().filter(|intersection| {intersection.impacted_collider == body_collider || intersection.impacted_collider == head_collider}) {
                player.handle_bullet_impact(&self.space, impact.clone());
            };
            
            
        }

        let mut enemy_iter = SwapIter::new(
            &mut self.enemies
        );

        while enemy_iter.not_done() {

            let (enemies, mut enemy) = enemy_iter.next();

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: &mut self.players,
                props: &mut self.props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies: enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue,
            };

            let body_collider = enemy.body.collider_handle;
            let head_collider = enemy.head.collider_handle;

            for impact in bullet_impact_queue.clone()
                .iter()
                .filter(
                    |intersection| 
                    {intersection.impacted_collider == body_collider || intersection.impacted_collider == head_collider}
                ) {
                enemy.handle_bullet_impact(
                    ctx,
                    &mut area_context,
                    impact.clone()
                );

                break;
            };
        }

        let mut prop_iter = SwapIter::new(
            &mut self.props
        );

        while prop_iter.not_done() {
            let (props, mut prop) = prop_iter.next();

            let collider = prop.collider_handle();

            let mut area_context = AreaContext {
                backgrounds: &mut self.backgrounds,
                spawn_point: &mut self.spawn_point,
                space: &mut self.space,
                decorations: &mut self.decorations,
                clips: &mut self.clips,
                players: &mut self.players,
                props: props,
                id: &mut self.id,
                bullet_trails: &mut self.bullet_trails,
                dissolved_pixels: &mut self.dissolved_pixels,
                enemies: &mut self.enemies,
                computer: &mut self.computer,
                dropped_items: &mut self.dropped_items,
                max_camera_y: &mut self.max_camera_y,
                minimum_camera_width: &mut self.minimum_camera_width,
                minimum_camera_height: &mut self.minimum_camera_height,
                despawn_y: &mut self.despawn_y,
                master: &mut self.master,
                ambiance: &mut self.ambiance,
                wave_data: &mut self.wave_data,
                compound_test: &mut self.compound_test,
                tiles: &mut self.tiles,
                impact_points: &mut self.impact_points,
                bullet_impact_queue: &mut self.bullet_impact_queue,
            };
            for impact in bullet_impact_queue.iter().filter(|impact| {impact.impacted_collider == collider}) {
                prop.handle_bullet_impact(
                    ctx, 
                    &mut area_context,
                    impact
                );
            };

            prop_iter.restore(prop);

        }
     

    }

    pub fn find_prop_mut(&mut self, id: PropId) -> Option<&mut Box<dyn Prop>> {
        if let Some(p) = self.props.iter_mut().find(|p| p.id() == id) {
            return Some(p);
        }
        
        // need to find the computer prop too

        None
    }

    pub fn from_save(
        save: AreaSave, 
        id: Option<AreaId>, 
        prefabs: &Prefabs,
        textures: TextureLoader
    ) -> Self {

        let mut space = Space::new();

        let mut decorations: Vec<Decoration> = Vec::new();
        let mut clips: Vec<Clip> = Vec::new();
        let mut players: Vec<Player> = Vec::new();
        let mut backgrounds: Vec<Background> = Vec::new();
        let mut generic_physics_props: Vec<Box<dyn Prop>> = Vec::new();
        let mut enemies: Vec<Enemy> = Vec::new();
        let mut dropped_items: Vec<DroppedItem> = Vec::new();
        let mut ambiance: Vec<Ambiance> = Vec::new();  
        let mut tiles: Vec<Vec<Option<Tile>>> = vec![vec![None; 100]; 10_000];
        
        for decoration_save in save.decorations {
            decorations.push(
                Decoration::from_save(decoration_save)
            );
        }

        for clip_save in save.clips {
            clips.push(
                Clip::from_save(clip_save, &mut space)
            );
        }

        for player_save in save.players {
            players.push(
                Player::from_save(player_save, &mut space, textures.clone())
            );
        }

        for background_save in save.backgrounds {
            backgrounds.push(
                Background::from_save(background_save)
            );
        }
        
        for generic_physics_prop in save.generic_physics_props {
            generic_physics_props.push(

                generic_physics_prop.load(&mut space, textures.clone())
            );
        }

        for enemy_save in save.enemies {
            enemies.push(
                Enemy::from_save(enemy_save, &mut space)
            );
        }

        for dropped_item_save in save.dropped_items {
            dropped_items.push(
                DroppedItem::from_save(dropped_item_save, &mut space, prefabs, textures.clone())
            );
        }

        for ambiance_save in save.ambiance {
            ambiance.push(
                Ambiance::from_save(ambiance_save)
            );
        }

        for tile_save in save.tiles {
            let tile = Tile::from_save(tile_save.clone());

            tiles[tile_save.position.0][tile_save.position.1] = Some(tile);
        }


        // if we are loading the id from the server we need to use the provided id
        let id = match id {
            Some(id) => id,
            None => AreaId::new(),
        };

        let computer = match save.computer_pos {
            Some(computer_pos) => {
                Some(Computer::new(prefabs, &mut space, computer_pos, textures))
            },
            None => None,
        };

        Self {
            spawn_point: save.spawn_point,
            space,
            decorations,
            clips,
            players,
            backgrounds,
            props: generic_physics_props,
            id,
            bullet_trails: Vec::new(), // we dont save bullet trails bsecause that'd be silly
            dissolved_pixels: Vec::new(), // same here
            enemies,
            computer,
            dropped_items,
            minimum_camera_width: save.minimum_camera_width,
            minimum_camera_height: save.minimum_camera_height,
            max_camera_y: save.max_camera_y,
            despawn_y: save.despawn_y,
            master: save.master,
            ambiance,
            wave_data: WaveData::default(),
            compound_test: Vec::new(),
            tiles,
            impact_points: Vec::new(),
            bullet_impact_queue: vec![]

        }
    }

    pub fn save(&self) -> AreaSave {

       
        let mut decorations: Vec<DecorationSave> = Vec::new();
        let mut clips: Vec<ClipSave> = Vec::new();
        let mut players: Vec<PlayerSave> = Vec::new();
        let mut backgrounds: Vec<BackgroundSave> = Vec::new();
        let mut generic_physics_props: Vec<Box<dyn PropSave>> = Vec::new();
        let mut enemies: Vec<EnemySave> = Vec::new();
        let mut dropped_items: Vec<DroppedItemSave> = Vec::new();
        let mut ambiances: Vec<AmbianceSave> = Vec::new();
        let tiles: Vec<TileSave> = vec![];
    
        for decoration in &self.decorations {
            decorations.push(
                decoration.save()
            );
        }

        for clip in &self.clips {
            clips.push(
                clip.save(&self.space)
            );
        }

        for player in &self.players {
            players.push(
                player.save(&self.space)
            );
        }

        for background in &self.backgrounds {
            backgrounds.push(
                background.save()
            );
        }

        for generic_physics_prop in &self.props {
            generic_physics_props.push(
                generic_physics_prop.save(&self.space)
            );
        }

        for enemy in &self.enemies {
            enemies.push(
                enemy.save(&self.space)
            );
        }

        for ambiance in &self.ambiance {
            ambiances.push (
                ambiance.save()
            )
        }

        

        let computer_pos = match &self.computer {
            Some(computer) => {
                Some(self.space.rigid_body_set.get(computer.prop.rigid_body_handle).unwrap().position().clone())
            },
            None => None,
        };

        for dropped_item in &self.dropped_items {
            dropped_items.push(dropped_item.save(&self.space))
        }

        AreaSave {
            spawn_point: self.spawn_point,
            decorations,
            clips,
            players,
            backgrounds,
            generic_physics_props,
            enemies,
            computer_pos,
            dropped_items,
            max_camera_y: self.max_camera_y,
            minimum_camera_width: self.minimum_camera_width,
            minimum_camera_height: self.minimum_camera_height,
            despawn_y: self.despawn_y,
            master: self.master,
            ambiance: ambiances,
            tiles

        }
    }

}


pub struct AreaContext<'a> {
    pub backgrounds: &'a mut Vec<Background>,
    pub spawn_point: &'a mut Vec2,
    pub space: &'a mut Space,
    pub decorations: &'a mut Vec<Decoration>,
    pub clips: &'a mut Vec<Clip>,
    pub players: &'a mut Vec<Player>,
    pub props: &'a mut Vec<Box<dyn Prop>>,
    pub id: &'a mut AreaId,
    pub bullet_trails: &'a mut Vec<BulletTrail>,
    pub dissolved_pixels: &'a mut Vec<DissolvedPixel>,
    pub enemies: &'a mut Vec<Enemy>,
    pub computer: &'a mut Option<Computer>,
    pub dropped_items: &'a mut Vec<DroppedItem>,
    pub max_camera_y: &'a mut f32,
    pub minimum_camera_width: &'a mut f32,
    pub minimum_camera_height: &'a mut f32,
    pub despawn_y: &'a mut f32,
    pub master: &'a mut Option<ClientId>,
    pub ambiance: &'a mut Vec<Ambiance>,
    pub wave_data: &'a mut WaveData,
    pub compound_test: &'a mut Vec<CompoundTest>,
    pub tiles: &'a mut Vec<Vec<Option<Tile>>>,
    pub impact_points: &'a mut Vec<glamx::Vec2>,
    pub bullet_impact_queue: &'a mut Vec<BulletImpactData>,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct AreaSave {
    spawn_point: Vec2,
    decorations: Vec<DecorationSave>,
    clips: Vec<ClipSave>,
    players: Vec<PlayerSave>,
    #[serde(default)]
    backgrounds: Vec<BackgroundSave>,
    #[serde(default)]
    generic_physics_props: Vec<Box<dyn PropSave>>,
    #[serde(default)]
    enemies: Vec<EnemySave>,
    #[serde(default)]
    computer_pos: Option<Pose2>,
    #[serde(default)]
    dropped_items: Vec<DroppedItemSave>,
    max_camera_y: f32,
    minimum_camera_width: f32,
    minimum_camera_height: f32,
    despawn_y: f32,
    #[serde(default)]
    master: Option<ClientId>,
    #[serde[default]]
    ambiance: Vec<AmbianceSave>,
    #[serde[default]]
    pub tiles: Vec<TileSave>
}