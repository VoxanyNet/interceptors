use std::{path::Path, time::Duration};

use macroquad::{color::WHITE, file::load_string, input::{is_key_pressed, is_key_released, KeyCode}, math::{Rect, Vec2}, miniquad::Backend, texture::{draw_texture_ex, load_texture, DrawTextureParams}, ui::Id, window::get_internal_gl};
use macroquad_tiled::{load_map, Map};
use nalgebra::{vector, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle};
use serde::{Deserialize, Serialize};

use crate::{background::{Background, BackgroundSave}, clip::{Clip, ClipSave}, decoration::{self, Decoration, DecorationSave}, player::{NewPlayer, Player, PlayerSave}, prop::{self, NewProp, Prop, PropSave}, rapier_mouse_world_pos, space::Space, texture_loader::TextureLoader, updates::NetworkPacket, uuid_u64, ClientTickContext, ServerIO};

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
    pub id: AreaId
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
            id: AreaId::new()
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect) {

        for background in &self.backgrounds {
            background.draw(textures, camera_rect).await
        }

        for decoration in &self.decorations {
            decoration.draw(textures).await
        }

        for generic_physics_prop in &self.props {
            generic_physics_prop.draw(&self.space, textures).await;
        }

        for player in &self.players {
            player.draw(&self.space, textures).await;
        }

    }


    pub fn server_tick(&mut self, io: &mut ServerIO, dt: web_time::Duration) {

    }

    pub fn spawn_player(&mut self, ctx: &mut ClientTickContext) {
        if is_key_released(KeyCode::F) {

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
    }


    pub fn spawn_prop(&mut self, ctx: &mut ClientTickContext) {

        if is_key_released(KeyCode::E) {
;
            
            let prefab_save: PropSave = serde_json::from_str(&ctx.prefabs.get_prefab_data("prefabs\\generic_physics_props\\brick_block.json")).unwrap();

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

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {

        self.spawn_prop(ctx);

        self.spawn_player(ctx);

        self.space.step(*ctx.last_tick_duration);

        for prop in &mut self.props {
            prop.client_tick(&mut self.space, self.id, ctx);
        }

        for player in &mut self.players {
            player.client_tick(ctx, &mut self.space, self.id);
        }
    }

    pub fn from_save(save: AreaSave, id: Option<AreaId>) -> Self {

        let mut space = Space::new();

        let mut decorations: Vec<Decoration> = Vec::new();
        let mut clips: Vec<Clip> = Vec::new();
        let mut players: Vec<Player> = Vec::new();
        let mut backgrounds: Vec<Background> = Vec::new();
        let mut generic_physics_props: Vec<Prop> = Vec::new();

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

        // if we are loading the id from the server we need to use the provided id
        let id = match id {
            Some(id) => id,
            None => AreaId::new(),
        };

        Self {
            spawn_point: save.spawn_point,
            space,
            decorations,
            clips,
            players,
            backgrounds,
            props: generic_physics_props,
            id
        }
    }

    pub fn save(&self) -> AreaSave {

        let mut decorations: Vec<DecorationSave> = Vec::new();
        let mut clips: Vec<ClipSave> = Vec::new();
        let mut players: Vec<PlayerSave> = Vec::new();
        let mut backgrounds: Vec<BackgroundSave> = Vec::new();
        let mut generic_physics_props: Vec<PropSave> = Vec::new();

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

        AreaSave {
            spawn_point: self.spawn_point,
            decorations,
            clips,
            players,
            backgrounds,
            generic_physics_props
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
    generic_physics_props: Vec<PropSave>
}