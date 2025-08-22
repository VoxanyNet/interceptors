use std::{path::PathBuf, str::FromStr, time::Instant};

use macroquad::{camera::{pop_camera_state, push_camera_state}, color::BLACK, math::Vec2, shapes::draw_rectangle, texture::RenderTarget};
use nalgebra::Isometry2;

use crate::{player::Player, prop::{Prop, PropItem, PropSave}, rapier_to_macroquad, texture_loader::TextureLoader, ClientTickContext, Prefabs};

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
            active: false
        }
    }
    
    pub fn tick(&mut self, ctx: &mut ClientTickContext, players: &mut Vec<Player>, space:&crate::space::Space) {
        let controlled_player = players.iter().find(|player| {player.owner == *ctx.client_id});

        let computer_pos = space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap();

        

        if let Some(controlled_player) = controlled_player {

            let player_pos = space.rigid_body_set.get(controlled_player.body.body_handle).unwrap().position();

            let controlled_player_distance = computer_pos.translation() - player_pos.translation.vector;

            dbg!(controlled_player_distance.magnitude());
            if controlled_player_distance.magnitude() > 200. {

                self.active = false;
                return;
            }

            self.active = true;
        }
    }
    pub async fn draw(&self, textures: &mut TextureLoader, space:&crate::space::Space, prefabs: &Prefabs) {
        self.prop.draw(space, textures).await;

        let prop_pos = space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap().position();

        let mut macroquad_pos = rapier_to_macroquad(prop_pos.translation.vector);

        

        if !self.active {
            return;
        }  


        let mut color = BLACK;

        color.a = 0.25;

        draw_rectangle(macroquad_pos.x - 200., macroquad_pos.y - 300., 400., 250., color);

        

        let selected_item = self.available_items.get(self.selected_item);

        macroquad_pos.y -= 60.;
        macroquad_pos.x -= 15.;

        if let Some(selected_item) = selected_item {
            selected_item.draw(textures, 1., macroquad_pos, prefabs);
        }  
    }
}