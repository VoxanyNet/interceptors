use macroquad::{camera::Camera2D, math::Rect};
use async_trait::async_trait;

use crate::{Prefabs, font_loader::FontLoader, space::Space, texture_loader::ClientTextureLoader, tile::Tile};

pub struct DrawContext<'a> {
    pub space: &'a Space,
    pub textures: &'a ClientTextureLoader,
    pub prefabs: &'a Prefabs,
    pub fonts: &'a FontLoader,
    pub camera_rect: &'a Rect,
    pub tiles: &'a Vec<Vec<Option<Tile>>>,
    pub elapsed_time: &'a web_time::Duration,
    pub default_camera: &'a Camera2D,
    pub editor: bool
}
#[async_trait]
pub trait Drawable {
    async fn draw(&mut self, draw_context: &DrawContext);

    fn draw_layer(&self) -> u32;
}