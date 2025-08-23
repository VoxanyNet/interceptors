use std::{path::{Path, PathBuf}, str::FromStr, time::Instant};

use macroquad::{camera::{pop_camera_state, push_camera_state, set_camera, set_default_camera, Camera2D}, color::{Color, BLACK, RED, WHITE}, math::{Rect, Vec2}, prelude::camera::mouse::Camera, shapes::draw_rectangle, text::{draw_text_ex, TextParams}, texture::{draw_texture, draw_texture_ex, render_target, DrawTextureParams, RenderTarget}, window::clear_background};
use nalgebra::Isometry2;

use crate::{font_loader::FontLoader, player::Player, prop::{Prop, PropItem, PropSave}, rapier_to_macroquad, texture_loader::TextureLoader, ClientTickContext, Prefabs};

pub enum Item {
    Prop(PropItem)
}

impl Item {
    pub fn draw_preview(&self, textures: &TextureLoader, draw_scale: f32, draw_pos: Vec2, prefabs: &Prefabs) {
        match self {
            Item::Prop(prop) => prop.draw_preview(textures, draw_scale, draw_pos, prefabs),
        }
    }
}
pub struct StoreItem {
    cost: u32,
    item: Item

}

impl StoreItem {
    pub fn draw(&self, textures: &TextureLoader, draw_scale: f32, draw_pos: Vec2, prefabs: &Prefabs) {
        self.item.draw_preview(textures, draw_scale, draw_pos, prefabs);
    }

}

pub struct Computer {
    pub available_items: Vec<StoreItem>,
    pub selected_item: usize,
    pub prop: Prop,
    pub active: bool,
    pub screen_pos: Vec2,
    pub screen_size: Vec2
}

impl Computer {

    pub fn new(prefabs: &Prefabs, space:&mut crate::space::Space, pos: Isometry2<f32> ) -> Self {
        
        let save: PropSave = serde_json::from_str(
            &prefabs.get_prefab_data("prefabs\\generic_physics_props\\computer.json")
        ).unwrap();

        let mut prop = Prop::from_save(
            save, 
            space
        );

        prop.set_pos(pos, space);

        let mut available_items = Vec::new();

        available_items.push(
            StoreItem {
                cost: 20,
                item: Item::Prop(
                    PropItem {
                        prefab_path: PathBuf::from_str("prefabs\\generic_physics_props\\box2.json").unwrap(),
                    }
                ),
            }
        );

        available_items.push(
            StoreItem {
                cost: 20,
                item: Item::Prop(
                    PropItem {
                        prefab_path: PathBuf::from_str("prefabs\\generic_physics_props\\anvil.json").unwrap(),
                    }
                ),
            }
        );

        Self {
            prop,
            available_items,
            selected_item: 0,
            active: false,
            screen_pos: Vec2::ONE,
            screen_size: Vec2::ONE
        }
    }
    
    pub fn tick(&mut self, ctx: &mut ClientTickContext, players: &mut Vec<Player>, space:&crate::space::Space) {
        let controlled_player = players.iter().find(|player| {player.owner == *ctx.client_id});

        let computer_pos = space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap();

        let mut macroquad_pos = rapier_to_macroquad(*computer_pos.translation());

        
        if let Some(controlled_player) = controlled_player {

            let player_pos = space.rigid_body_set.get(controlled_player.body.body_handle).unwrap().position();

            let controlled_player_distance = computer_pos.translation() - player_pos.translation.vector;

            if controlled_player_distance.magnitude() > 200. {

                self.active = false;
                return;
            }

            self.active = true;
        }
    }
    pub async fn draw(&self, textures: &mut TextureLoader, space:&crate::space::Space, prefabs: &Prefabs, default_camera: &Camera2D, fonts: &FontLoader) {
        self.prop.draw(space, textures).await;

        let prop_pos = space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap().position();

        let mut macroquad_pos = rapier_to_macroquad(prop_pos.translation.vector);

        

        if !self.active {
            return;
        }  


        let mut color = BLACK;

        color.a = 0.25;

        let render_target = render_target(320, 180);

        let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 320., 180.));

        camera.render_target = Some(render_target);

        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

    
        clear_background(color);        

        let font = fonts.get(PathBuf::from("assets/fonts/CutePixel.ttf"));

        //draw_rectangle(0., 0., 20., 20., RED);

        draw_text_ex("STORE", 0., 20., TextParams {
            font: Some(&font),
            font_size: 32,
            color: WHITE,
            ..Default::default()
            
        });
        

        let selected_item = self.available_items.get(self.selected_item);

        // let preview_draw_pos = Vec2 {
        //     x: macroquad_pos.x - 15.,
        //     y: macroquad_pos.y - 60.,
        // };

        // if let Some(selected_item) = selected_item {
        //     selected_item.draw(textures, 1., preview_draw_pos, prefabs);
        // } 

        // set the camera back
        set_camera(default_camera);

        draw_texture(&camera.render_target.unwrap().texture, macroquad_pos.x - 160., macroquad_pos.y - 180., WHITE);
    }
}