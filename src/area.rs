
use std::{path::PathBuf, time::Instant};

use macroquad::{audio::{play_sound, stop_sound, PlaySoundParams}, camera::Camera2D, input::{is_key_released, KeyCode}, math::Rect};
use nalgebra::{vector, Isometry2, Vector2};
use rapier2d::prelude::RigidBodyVelocity;
use serde::{Deserialize, Serialize};

use crate::{ambiance::{Ambiance, AmbianceSave}, background::{Background, BackgroundSave}, bullet_trail::BulletTrail, car::Car, clip::{Clip, ClipSave}, compound_test::CompoundTest, computer::Computer, decoration::{Decoration, DecorationSave}, dropped_item::{DroppedItem, DroppedItemSave, NewDroppedItemUpdate}, enemy::{Enemy, EnemySave, NewEnemyUpdate}, font_loader::FontLoader, player::{NewPlayer, Player, PlayerSave}, prop::{DissolvedPixel, NewProp, Prop, PropId, PropSave}, rapier_mouse_world_pos, space::Space, texture_loader::TextureLoader, updates::{MasterUpdate, NetworkPacket}, uuid_u64, weapons::{shotgun::item::ShotgunItem, weapon_type_item::WeaponTypeItem}, ClientId, ClientTickContext, Prefabs, ServerIO, SwapIter};

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
    pub spawn_point: Vector2<f32>,
    pub space: Space,
    pub decorations: Vec<Decoration>,
    pub clips: Vec<Clip>,
    pub players: Vec<Player>,
    pub props: Vec<Prop>,
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
    pub cars: Vec<Car>,
    pub compound_test: Vec<CompoundTest>
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
    pub fn empty() -> Self {

        
        Self {
            spawn_point: Vector2::zeros(),
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
            cars: Vec::new(),
            compound_test: Vec::new()
        }
    }

    pub async fn draw(&mut self, textures: &mut TextureLoader, camera_rect: &Rect, prefabs: &Prefabs, camera: &Camera2D, fonts: &FontLoader, elapsed: web_time::Duration) {

        for background in &self.backgrounds {
            background.draw(textures, camera_rect).await
        }

        for decoration in &self.decorations {
            decoration.draw(textures, elapsed).await
        }

        for generic_physics_prop in &self.props {
            generic_physics_prop.draw(&self.space, textures).await;
        }

        for dropped_item in &self.dropped_items {
            dropped_item.draw(&self.space, textures, prefabs);
        }
        if let Some(computer) = &mut self.computer {
            computer.draw(textures, &self.space, prefabs, camera, fonts).await;
        }

        for player in &self.players {
            player.draw(&self.space, textures, prefabs, fonts).await;
        }

        for enemy in &self.enemies {
            enemy.draw(&self.space, textures).await;
        }

        for car in &self.cars {
            car.draw(&self.space);
        }

        for pixel in &self.dissolved_pixels {
            pixel.draw(&self.space);
        }

        for bullet_trail in &self.bullet_trails {
            bullet_trail.draw();
        }

        for compound_test in &self.compound_test {
            compound_test.draw(&self.space, textures);
        }


    }

    pub fn draw_hud(&self, textures: &TextureLoader) {
        for player in &self.players {
            player.draw_hud(textures);
        }
    }


    pub fn server_tick(&mut self, io: &mut ServerIO, dt: web_time::Duration) {
        self.space.step(dt);

        self.designate_master(io);
    }

    pub fn designate_master(&mut self, server_io: &mut ServerIO) {
        if self.master == None {
            if let Some(player) = self.players.get(0) {
                self.master = Some(player.owner);

                server_io.send_all_clients(
                    NetworkPacket::MasterUpdate(
                        MasterUpdate {
                            area_id: self.id,
                            master: self.master.unwrap().clone(),
                        }
                    ), 
                );
            }
        }
    }

    pub fn spawn_player(&mut self, ctx: &mut ClientTickContext) {

        let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

        let player = Player::new(vector![mouse_pos.x, mouse_pos.y].into(), &mut self.space, *ctx.client_id);

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


    pub fn spawn_prop(&mut self, ctx: &mut ClientTickContext) {

        if is_key_released(KeyCode::E) {
            
            let prefab_save: PropSave = serde_json::from_str(&ctx.prefabs.get_prefab_data("prefabs\\generic_physics_props\\box2.json")).unwrap();

            let mut new_prop = Prop::from_save(prefab_save, &mut self.space);

            new_prop.owner = Some(*ctx.client_id);

            let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

            new_prop.set_pos(vector![mouse_pos.x, mouse_pos.y].into(), &mut self.space);


            ctx.network_io.send_network_packet(
                NetworkPacket::NewProp(
                    NewProp
                    {
                        prop: new_prop.save(&mut self.space), 
                        area_id: self.id
                    }
                )
            );

            self.props.push(new_prop);
        }
    }

    pub fn spawn_enemy(&mut self, ctx: &mut ClientTickContext) {
        let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);
        
        let enemy = Enemy::new( Isometry2::new(mouse_pos, 0.), *ctx.client_id, &mut self.space, Some(WeaponTypeItem::Shotgun(ShotgunItem::new())));

        dbg!(enemy.id);

        ctx.network_io.send_network_packet(crate::updates::NetworkPacket::NewEnemyUpdate(
            NewEnemyUpdate {
                area_id: self.id,
                enemy: enemy.save(&mut self.space),
            }
        ));

        self.enemies.push(enemy);
    }

    pub fn average_enemy_pos(&self, space: &Space) -> Option<Vector2<f32>> {

        if self.enemies.len() == 0 {
            return None
        }

        let mut cumulative_x = 0.;
        let mut cumulative_y = 0.;

        for enemy in &self.enemies {
  
            let enemy_pos = space.rigid_body_set.get(enemy.body.body_handle).unwrap().position().translation;

            cumulative_x += enemy_pos.x;
            cumulative_y += enemy_pos.y;
        }   

        Some(
            Vector2::new(
                cumulative_x / self.enemies.len() as f32,
                cumulative_y / self.enemies.len() as f32
            )
        )

    }

    // the player should be spawned by the server - this is temporary
    pub fn spawn_player_if_not_in_game(&mut self, ctx: &mut ClientTickContext) {

        match self.players.iter().find(|player| player.owner == *ctx.client_id) {
            Some(_) => return,
            None => {

                let player = Player::new(self.spawn_point.into(), &mut self.space, *ctx.client_id);

                ctx.network_io.send_network_packet(
                    NetworkPacket::NewPlayer(
                        NewPlayer {
                            player: player.save(&self.space),
                            area_id: self.id,
                        }
                    )
                );

                self.players.push(
                    player
                );

                
            },
        }
    }

    pub fn start_ambiance(&mut self, ctx: &mut ClientTickContext) {
        for ambiance in &mut self.ambiance {
            if ambiance.sound.is_none() {

                let sound = ctx.sounds.get(ambiance.path.clone());
                play_sound(
                    sound, 
                    PlaySoundParams {
                        looped: true,
                        volume: ambiance.volume,
                    }
                );

                ambiance.sound = Some(sound.clone());
            }


        }
    }

    pub fn stop_ambiance(&mut self) {
        if is_key_released(KeyCode::J) {
            for ambiance in &mut self.ambiance {
                stop_sound(ambiance.sound.as_ref().unwrap());

                
            }
        }
    }


    pub fn wave_logic(&mut self, ctx: &mut ClientTickContext) {

        // end wave
        if self.wave_data.spawned_this_wave >= self.wave_data.total_size {
            self.wave_data.wave_end = web_time::Instant::now();

            self.wave_data.spawned_this_wave = 0;

            self.wave_data.active = false;

            dbg!("ending wave");

            
        }

        // start new wave
        if self.wave_data.wave_end.elapsed().as_secs_f32() > 5. && self.wave_data.active == false  {

            dbg!("starting wave");

            self.wave_data.wave += 1;

            self.wave_data.batch_size += 2;

            self.wave_data.total_size += 10;

            self.wave_data.active = true;

            self.wave_data.wave_start = web_time::Instant::now();
        }

        // spawn batch
        if self.wave_data.active && self.wave_data.last_batch_spawn.elapsed() > self.wave_data.batch_interval && self.enemies.len() == 0 {

            dbg!("spawning batch");

            for i in 0..self.wave_data.batch_size {

                let enemy = Enemy::new(
                    vector![2400. + (i as f32 * 50.), 200.].into(), 
                    self.master.unwrap().clone(), 
                    &mut self.space, 
                    Some(WeaponTypeItem::Shotgun(
                        ShotgunItem::new()
                    ))
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

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {

        if is_key_released(KeyCode::M) {
            self.cars.push(
                Car::new(&mut self.space, rapier_mouse_world_pos(&ctx.camera_rect).into())
            );
        }

        if is_key_released(KeyCode::I) {
            self.compound_test.push(
                CompoundTest::new(&mut self.space, ctx, PathBuf::from("assets\\stone1.png"), 0., 0., rapier_mouse_world_pos(&ctx.camera_rect))
            );
        }

        //self.wave_logic(ctx);

        self.start_ambiance(ctx);

        self.stop_ambiance();

        self.spawn_player_if_not_in_game(ctx);

        self.spawn_dropped_item(ctx);

        if is_key_released(KeyCode::T) {
            self.spawn_enemy(ctx);
        }


        self.spawn_prop(ctx);


        // if is_key_down(KeyCode::J) {
        //     self.space.step(Duration::from_secs_f32(1./60.));
        // }

        //let then = Instant::now();
        self.space.step(*ctx.last_tick_duration);
        //dbg!(then.elapsed());


        for prop in &mut self.props {
            prop.client_tick(&mut self.space, self.id, ctx, &mut self.dissolved_pixels);
        }

        self.props.retain(|prop| {prop.despawn == false});

        let mut enemy_iter = SwapIter::new(&mut self.enemies);

        let then = Instant::now();

        while enemy_iter.not_done() {
            let (enemies, mut enemy) = enemy_iter.next();

            

            

            enemy.client_tick(
                &mut self.space, 
                ctx, 
                &mut self.players, 
                self.despawn_y,
                &mut self.props,
                &mut self.bullet_trails,
                self.id,
                &mut self.dissolved_pixels,
                enemies
            );

            enemy_iter.restore(enemy);

            

           
        }


        //dbg!(then.elapsed());

        


        self.enemies.retain(|enemy| {enemy.despawn == false});

        for dissolved_pixel in &mut self.dissolved_pixels {
            dissolved_pixel.client_tick(&mut self.space, ctx);
        }

        self.dissolved_pixels.retain(|pixel| {pixel.despawn == false});


        for bullet_trail in &mut self.bullet_trails {
            bullet_trail.client_tick(ctx);
        }

        if is_key_released(KeyCode::F) {
            self.spawn_player(ctx);
        }   

        let average_enemy_pos = self.average_enemy_pos(&self.space); 

        let mut players_iter = SwapIter::new(&mut self.players);
        
        while players_iter.not_done() {
            let (players, mut player) = players_iter.next();

            player.client_tick(
                ctx, 
                &mut self.space, 
                self.id, 
                players, 
                &mut self.enemies,
                &mut self.props, 
                &mut self.bullet_trails,
                &mut self.dissolved_pixels, 
                &mut self.dropped_items,
                self.max_camera_y, 
                average_enemy_pos, 
                self.minimum_camera_width, 
                self.minimum_camera_height
            );

            players_iter.restore(player);
        }


        if let Some(computer) = &mut self.computer {
            computer.tick(ctx, &mut self.players, &self.space);
        }
    }

    pub fn find_prop_mut(&mut self, id: PropId) -> Option<&mut Prop> {
        if let Some(p) = self.props.iter_mut().find(|p| p.id == id) {
            return Some(p);
        }
        if let Some(c) = &mut self.computer {

            if c.prop.id == id {
                return Some(&mut c.prop)
            }
            
        }
        None
    }

    pub fn spawn_dropped_item(&mut self, ctx: &mut ClientTickContext) {
        if is_key_released(KeyCode::K) {

            
            let mouse_pos = rapier_mouse_world_pos(&ctx.camera_rect);

            let dropped_item = DroppedItem::new(
                crate::computer::Item::Weapon(
                    WeaponTypeItem::Shotgun(
                        ShotgunItem::new()
                    )
                ),
                mouse_pos.into(), 
                RigidBodyVelocity::zero(), 
                &mut self.space, 
                ctx.textures, 
                ctx.prefabs,
                20.
            );

            self.dropped_items.push(
                dropped_item.clone()
            );

            ctx.network_io.send_network_packet(
            NetworkPacket::NewDroppedItemUpdate(
                NewDroppedItemUpdate {
                    dropped_item: dropped_item.save(&self.space),
                    area_id: self.id,
                }
            )
        );
        }

        
    }

    pub fn from_save(save: AreaSave, id: Option<AreaId>, prefabs: &Prefabs) -> Self {

        let mut space = Space::new();

        let mut decorations: Vec<Decoration> = Vec::new();
        let mut clips: Vec<Clip> = Vec::new();
        let mut players: Vec<Player> = Vec::new();
        let mut backgrounds: Vec<Background> = Vec::new();
        let mut generic_physics_props: Vec<Prop> = Vec::new();
        let mut enemies: Vec<Enemy> = Vec::new();
        let mut dropped_items: Vec<DroppedItem> = Vec::new();
        let mut ambiance: Vec<Ambiance> = Vec::new();  
        
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
                Player::from_save(player_save, &mut space)
            );
        }

        for background_save in save.backgrounds {
            backgrounds.push(
                Background::from_save(background_save)
            );
        }
        
        for generic_physics_prop in save.generic_physics_props {
            generic_physics_props.push(
                Prop::from_save(generic_physics_prop, &mut space)
            );
        }

        for enemy_save in save.enemies {
            enemies.push(
                Enemy::from_save(enemy_save, &mut space)
            );
        }

        for dropped_item_save in save.dropped_items {
            dropped_items.push(
                DroppedItem::from_save(dropped_item_save, &mut space, prefabs)
            );
        }

        for ambiance_save in save.ambiance {
            ambiance.push(
                Ambiance::from_save(ambiance_save)
            );
        }


        // if we are loading the id from the server we need to use the provided id
        let id = match id {
            Some(id) => id,
            None => AreaId::new(),
        };

        let computer = match save.computer_pos {
            Some(computer_pos) => {
                Some(Computer::new(prefabs, &mut space, computer_pos, ))
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
            cars: Vec::new(),
            compound_test: Vec::new()

        }
    }

    pub fn save(&self) -> AreaSave {

        let mut decorations: Vec<DecorationSave> = Vec::new();
        let mut clips: Vec<ClipSave> = Vec::new();
        let mut players: Vec<PlayerSave> = Vec::new();
        let mut backgrounds: Vec<BackgroundSave> = Vec::new();
        let mut generic_physics_props: Vec<PropSave> = Vec::new();
        let mut enemies: Vec<EnemySave> = Vec::new();
        let mut dropped_items: Vec<DroppedItemSave> = Vec::new();
        let mut ambiances: Vec<AmbianceSave> = Vec::new();

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
            ambiance: ambiances

        }
    }


}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AreaSave {
    spawn_point: Vector2<f32>,
    decorations: Vec<DecorationSave>,
    clips: Vec<ClipSave>,
    players: Vec<PlayerSave>,
    #[serde(default)]
    backgrounds: Vec<BackgroundSave>,
    #[serde(default)]
    generic_physics_props: Vec<PropSave>,
    #[serde(default)]
    enemies: Vec<EnemySave>,
    #[serde(default)]
    computer_pos: Option<Isometry2<f32>>,
    #[serde(default)]
    dropped_items: Vec<DroppedItemSave>,
    max_camera_y: f32,
    minimum_camera_width: f32,
    minimum_camera_height: f32,
    despawn_y: f32,
    #[serde(default)]
    master: Option<ClientId>,
    #[serde[default]]
    ambiance: Vec<AmbianceSave>
}