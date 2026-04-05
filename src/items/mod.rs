use macroquad::{color::Color, math::Vec2};

use crate::{Prefabs, space::{self, Space}, texture_loader::ClientTextureLoader};

pub mod prop;

#[typetag::serde(tag = "type")]
pub trait Item {
    fn stackable(&self) -> bool;
    fn save(&self, space: &Space) -> ItemSave;
    fn draw_preview(
        &self, 
        textures: &ClientTextureLoader, 
        size: f32,
        draw_pos: Vec2,
        color: Option<Color>,
        rotation: f32
    );
    fn get_preview_resolution(
        &self,
        textures: &ClientTextureLoader,
        size: f32
    ) -> Vec2;

    fn name(&self) -> String;
}