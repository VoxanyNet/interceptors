
use std::{path::{Path, PathBuf}, time::{Duration, Instant}};

use macroquad::{camera::Camera2D, input::{is_key_down, is_key_released, KeyCode}, math::Rect};
use nalgebra::{vector, Isometry, Isometry2, Vector2};
use rapier2d::prelude::RigidBodyVelocity;
use serde::{Deserialize, Serialize};
use web_sys::js_sys::WebAssembly::Instance;

use crate::{background::{Background, BackgroundSave}, bullet_trail::BulletTrail, clip::{Clip, ClipSave}, computer::Computer, decoration::{Decoration, DecorationSave}, dropped_item::{DroppedItem, DroppedItemSave, NewDroppedItemUpdate}, enemy::{Enemy, EnemySave}, font_loader::FontLoader, player::{NewPlayer, Player, PlayerSave}, prop::{DissolvedPixel, NewProp, Prop, PropId, PropItem, PropSave}, rapier_mouse_world_pos, shotgun::{Shotgun, ShotgunItem}, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, ClientTickContext, Prefabs, ServerIO, SwapIter};

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
    pub dropped_items: Vec<DroppedItem>
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
            dropped_items: Vec::new()
        }
    }

    pub async fn draw(&mut self, textures: &mut TextureLoader, camera_rect: &Rect, prefabs: &Prefabs, camera: &Camera2D, fonts: &FontLoader) {

        for background in &self.backgrounds {
            background.draw(textures, camera_rect).await
        }

        for decoration in &self.decorations {
            decoration.draw(textures).await
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

        for pixel in &self.dissolved_pixels {
            pixel.draw(&self.space);
        }

        for bullet_trail in &self.bullet_trails {
            bullet_trail.draw();
        }


    }

    pub fn draw_hud(&self, textures: &TextureLoader) {
        for player in &self.players {
            player.draw_hud(textures);
        }
    }


    pub fn server_tick(&mut self, io: &mut ServerIO, dt: web_time::Duration) {
        self.space.step(dt);
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
        
        let enemy = Enemy::new(Isometry2::new(mouse_pos, 0.), *ctx.client_id, &mut self.space);

        self.enemies.push(enemy);
    }

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {

        self.spawn_dropped_item(ctx);

        if is_key_released(KeyCode::T) {
            self.spawn_enemy(ctx);
        }
        self.spawn_prop(ctx);


        // if is_key_down(KeyCode::J) {
        //     self.space.step(Duration::from_secs_f32(1./60.));
        // }

        self.space.step(*ctx.last_tick_duration);


        for prop in &mut self.props {
            prop.client_tick(&mut self.space, self.id, ctx, &mut self.dissolved_pixels);
        }

        self.props.retain(|prop| {prop.despawn == false});

        for enemy in &mut self.enemies {
            enemy.client_tick(&mut self.space, ctx, &self.players);
        }

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

        let mut players_iter = SwapIter::new(&mut self.players);

        while players_iter.not_done() {
            let (players, mut player) = players_iter.next();

            player.client_tick(ctx, &mut self.space, self.id, players, &mut self.props, &mut self.bullet_trails, &mut self.dissolved_pixels, &mut self.dropped_items);

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
                    crate::weapon::WeaponTypeItem::Shotgun(
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


        // if we are loading the id from the server we need to use the provided id
        let id = match id {
            Some(id) => id,
            None => AreaId::new(),
        };

        let computer = match save.computer_pos {
            Some(computer_pos) => {
                Some(Computer::new(prefabs, &mut space, computer_pos, ))
            },
            None => Some(Computer::new(prefabs, &mut space, vector![650., 120.].into())),
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
            dropped_items

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
            dropped_items
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
    dropped_items: Vec<DroppedItemSave>
}