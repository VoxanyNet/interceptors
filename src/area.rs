use macroquad::{color::WHITE, file::load_string, math::{Rect, Vec2}, miniquad::Backend, texture::{draw_texture_ex, load_texture, DrawTextureParams}, window::get_internal_gl};
use macroquad_tiled::{load_map, Map};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle};
use serde::{Deserialize, Serialize};

use crate::{background::{Background, BackgroundSave}, clip::{Clip, ClipSave}, decoration::{self, Decoration, DecorationSave}, generic_physics_prop::{self, GenericPhysicsProp, GenericPhysicsPropSave}, player::{Player, PlayerSave}, prop::{Prop, PropSave}, space::Space, texture_loader::TextureLoader, ClientTickContext};

pub struct Area {
    pub backgrounds: Vec<Background>,
    pub spawn_point: Vec2,
    pub space: Space,
    pub decorations: Vec<Decoration>,
    pub clips: Vec<Clip>,
    pub players: Vec<Player>,
    pub generic_physics_props: Vec<GenericPhysicsProp>
}

impl Area {
    pub fn empty() -> Self {

        
        Self {
            spawn_point: Vec2::ZERO,
            space: Space::new(),
            decorations: Vec::new(),
            clips: Vec::new(),
            players: Vec::new(),
            backgrounds: Vec::new(),
            generic_physics_props: Vec::new()
        }
    }

    pub async fn draw(&self, textures: &mut TextureLoader, camera_rect: &Rect) {

        for background in &self.backgrounds {
            background.draw(textures, camera_rect).await
        }

        for decoration in &self.decorations {
            decoration.draw(textures).await
        }

        for generic_physics_prop in &self.generic_physics_props {
            generic_physics_prop.draw(&self.space, textures).await;
        }

    }


    pub fn server_tick(&mut self) {
        
    }

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        
    }

    pub fn from_save(save: AreaSave) -> Self {

        let mut space = Space::new();

        let mut decorations: Vec<Decoration> = Vec::new();
        let mut clips: Vec<Clip> = Vec::new();
        let mut players: Vec<Player> = Vec::new();
        let mut backgrounds: Vec<Background> = Vec::new();
        let mut generic_physics_props: Vec<GenericPhysicsProp> = Vec::new();

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
                Player::from_save(player_save)
            );
        }

        for background_save in save.backgrounds {
            backgrounds.push(
                Background::from_save(background_save)
            );
        }
        
        for generic_physics_prop in save.generic_physics_props {
            generic_physics_props.push(
                GenericPhysicsProp::from_save(generic_physics_prop, &mut space)
            );
        }
        Self {
            spawn_point: save.spawn_point,
            space,
            decorations,
            clips,
            players,
            backgrounds,
            generic_physics_props
        }
    }

    pub fn save(&self) -> AreaSave {

        let mut decorations: Vec<DecorationSave> = Vec::new();
        let mut clips: Vec<ClipSave> = Vec::new();
        let mut players: Vec<PlayerSave> = Vec::new();
        let mut backgrounds: Vec<BackgroundSave> = Vec::new();
        let mut generic_physics_props: Vec<GenericPhysicsPropSave> = Vec::new();

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
                player.save()
            );
        }

        for background in &self.backgrounds {
            backgrounds.push(
                background.save()
            );
        }

        for generic_physics_prop in &self.generic_physics_props {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct AreaSave {
    spawn_point: Vec2,
    decorations: Vec<DecorationSave>,
    clips: Vec<ClipSave>,
    players: Vec<PlayerSave>,
    #[serde(default)]
    backgrounds: Vec<BackgroundSave>,
    #[serde(default)]
    generic_physics_props: Vec<GenericPhysicsPropSave>
}