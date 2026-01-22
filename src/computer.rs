use std::path::PathBuf;

use derive_more::From;
use glamx::Pose2;
use macroquad::{camera::{set_camera, Camera2D}, color::{Color, BLACK, GRAY, WHITE}, math::{Rect, Vec2}, shapes::draw_line, text::{draw_text_ex, TextParams}, texture::{draw_texture_ex, render_target, DrawTextureParams, RenderTarget}, window::clear_background};
use serde::{Deserialize, Serialize};

use crate::{ClientTickContext, Owner, Prefabs, button::Button, drawable::{DrawContext, Drawable}, font_loader::FontLoader, mouse_world_pos, player::Player, prop::{Prop, PropSave}, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, weapons::{weapon_type::WeaponType, weapon_type_save::WeaponTypeSave}};

#[derive(PartialEq, Clone, Debug, From)]
pub enum Item {
    Prop(Prop),
    Weapon(WeaponType)
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ItemSave {
    Prop(PropSave),
    Weapon(WeaponTypeSave)
}



impl Item {

    pub fn stackable(&self) -> bool {
        match self {
            Item::Prop(prop) => true,
            Item::Weapon(weapon_type_item) => false,
        }
    }

    pub fn save(&self, space: &Space) -> ItemSave {
        match self {
            Item::Prop(prop) => ItemSave::Prop(prop.save(space)),
            Item::Weapon(weapon) => ItemSave::Weapon(weapon.save(space))
        }
    }

    pub fn from_save(item_save: ItemSave, space: &mut Space) -> Item {
        match item_save {
            ItemSave::Prop(prop_save) => Item::Prop(Prop::from_save(prop_save, space)),
            ItemSave::Weapon(weapon_type_save) => Item::Weapon(WeaponType::from_save(space, weapon_type_save, None)) 
        }
    }
    pub fn draw_preview(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, prefabs: &Prefabs, color: Option<Color>, rotation: f32) {
        match self {
            Item::Prop(prop) => prop.draw_preview(textures, size, draw_pos, prefabs, color, rotation),
            Item::Weapon(weapon) => weapon.draw_preview(textures, size, draw_pos, color, rotation),
        }
    }

    pub fn get_preview_resolution(&self, textures: &TextureLoader, size: f32, prefabs: &Prefabs) -> Vec2 {
        match self {
            Item::Prop(prop_item) => prop_item.get_preview_resolution(size, prefabs, textures),
            Item::Weapon(weapon) => weapon.get_preview_resolution(size, textures)
            
        }
    }

    pub fn name(&self, prefabs: &Prefabs) -> String {
        match self {
            Item::Prop(prop) => prop.name(),
            Item::Weapon(weapon) => weapon.name()
            
        }
    }
}
pub struct StoreItem {
    cost: u32,
    item: Item,
    quantity: Option<u32>

}

impl StoreItem {
    pub fn draw(&self, textures: &TextureLoader, size: f32, draw_pos: Vec2, prefabs: &Prefabs, color: Option<Color>, rotation: f32) {
        self.item.draw_preview(textures, size, draw_pos, prefabs, color, rotation);
    }

}

pub struct CategoryTab {
    button: Button,
    text: String,
    font: PathBuf,
    active: bool
}

impl CategoryTab {

    pub fn new(text: impl ToString, pos: Vec2, font: PathBuf) -> Self {

        let text_length = text.to_string().len() as f32;
        Self {
            button: Button::new(
                Rect {
                    x: pos.x,
                    y: pos.y,
                    w: text_length * 13.,
                    h: 20.,
                },
                None
            ),
            text: text.to_string(),
            font,
            active: false
        }
    }
    pub fn update(&mut self, mouse_pos: Vec2) {
        self.button.update(mouse_pos);
    }

    pub fn draw(&self, fonts: &FontLoader) {

        // draw_rectangle(self.button.rect.x, self.button.rect.y, self.button.rect.w, self.button.rect.h, Color {
        //     r: 1.,
        //     g: 1.,
        //     b: 1.,
        //     a: 0.75,
        // });

        let color = match self.button.hovered {
            true => {
                WHITE
            },
            false => {
                match self.active {
                    true => WHITE,
                    false => GRAY,
                }
            },
        };



        let text_length = self.text.len() as f32;

        draw_text_ex(
            &self.text, 
            self.button.rect.x, 
            self.button.rect.y + 20., 
            TextParams {
                font: Some(&fonts.get(self.font.clone())),
                font_size: 32,
                color,
                ..Default::default()
            }
        );

        if self.active {
            draw_line(self.button.rect.left(), self.button.rect.bottom() + 3., self.button.rect.left() + text_length * 13. + 3., self.button.rect.bottom() + 3., 2., color);
        }
        
    }
}

pub struct StoreCategory {
    pub items: Vec<StoreItem>,
    pub item_select_buttons: Vec<Button>,
    pub pos: Vec2
}

impl StoreCategory {

    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            item_select_buttons: Vec::new(),
            pos: Vec2::new(10., 35.)
        }
    }

    pub fn insert_item(&mut self, item: StoreItem) {

        let pos = Vec2 {
            x: (40. * (self.items.len() % 7) as f32) + self.pos.x,
            y: (40. * (self.items.len() / 7) as f32) + self.pos.y,
        };

        self.items.push(item);

        self.item_select_buttons.push(
            Button::new(
                Rect::new(
                    pos.x, 
                    pos.y, 
                    40., 
                    40.
                ),
                None
            )
        );
    }

    pub fn tick(&mut self, mouse_pos: Vec2, ) {
        for button in &mut self.item_select_buttons {
            button.update(mouse_pos);
        }
    }

    pub fn draw(&self, textures: &TextureLoader, prefabs: &Prefabs, fonts: &FontLoader) {

        let hovered_button_color = Color::new(0.78, 0.78, 0.78, 0.5);

        for (index, item) in self.items.iter().enumerate() {

            let color = match self.item_select_buttons.get(index).unwrap().hovered {
                true => WHITE,
                false => Color::new(1.00, 1.00, 1.00, 0.8),
            };

            let draw_pos = Vec2 {
                x: (40. * (index % 7) as f32) + self.pos.x,
                y: (40. * (index / 7) as f32) + self.pos.y,
            };

            item.draw(
                textures, 
                36., 
                draw_pos, 
                prefabs,
                Some(color),
                0.
            );

            if let Some(quantity) = item.quantity {
                draw_text_ex(
                    &quantity.to_string(), 
                    draw_pos.x, 
                    draw_pos.y + 24., 
                    TextParams {
                        font: Some(&fonts.get(PathBuf::from("assets/fonts/CutePixel.ttf"))),
                        font_size: 24,
                        color: WHITE,
                        ..Default::default()
                    }
                );
            };
        }
    }
}
pub struct Computer {
    pub available_items: Vec<StoreItem>,
    pub selected_item: usize,
    pub prop: Prop,
    pub active: bool,
    pub screen_pos: Vec2,
    pub screen_size: Vec2,
    pub activated_time: web_time::Instant,
    pub category_tabs: Vec<CategoryTab>,
    pub selected_category: usize,
    pub item_categories: Vec<StoreCategory>,
    pub render_target: Option<RenderTarget> // server cant initialize render targets 
}


impl Computer {

    pub fn new(prefabs: &Prefabs, space:&mut crate::space::Space, pos: glamx::Pose2) -> Self {
        
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
                    Prop::from_prefab("prefabs\\generic_physics_props\\box2.json".to_string(), space)
                ),
                quantity: None
            }
        );

        available_items.push(
            StoreItem {
                cost: 20,
                item: Item::Prop(Prop::from_prefab("prefabs\\generic_physics_props\\anvil.json".to_string(), space)),
                quantity: None
            }
        );

        let mut category_tabs = Vec::new();

        category_tabs.push(
            CategoryTab::new(
                "STRU", 
                Vec2::new(0., 0.), 
                "assets/fonts/CutePixel.ttf".into()
                
            )
        );

        category_tabs.push(
            CategoryTab::new(
                "WEAP", 
                Vec2::new(65., 0.), 
                "assets/fonts/CutePixel.ttf".into()
                
            )
        );

        let mut item_categories: Vec<StoreCategory> = Vec::new();

        let mut structures_category = StoreCategory::new();

        for _ in 0..10 {
            structures_category.insert_item(
                StoreItem {
                    cost: 20,
                    item: Item::Prop(
                        Prop::from_prefab("prefabs\\generic_physics_props\\box2.json".to_string(), space)
                    ),
                    quantity: None
                }
            );
        }

        for _ in 0..10 {
            structures_category.insert_item(
                StoreItem {
                    cost: 20,
                    item: Item::Prop(
                        Prop::from_prefab("prefabs\\generic_physics_props\\stone2.json".to_string(), space)
                    ),
                    quantity: Some(1)
                }
            );
        }

        item_categories.push(structures_category);
        
        

        Self {
            prop,
            available_items,
            selected_item: 0,
            active: false,
            screen_pos: Vec2::ONE,
            screen_size: Vec2::ONE,
            activated_time: web_time::Instant::now(),
            category_tabs,
            selected_category: 0,
            item_categories,
            render_target: None
        }
    }

    
    pub fn tick(&mut self, ctx: &mut ClientTickContext, players: &mut Vec<Player>, space:&crate::space::Space) {


        let mouse_pos = self.get_mouse_pos(&ctx.camera_rect);

        for (index, tab) in self.category_tabs.iter_mut().enumerate() {
            tab.update(mouse_pos);

            if tab.button.released {
                self.selected_category = index;
            }
        }

        // need to inform the rest of the tabs that they are not active 
        for (index, tab) in self.category_tabs.iter_mut().enumerate() {
            if index != self.selected_category {
                tab.active = false;
            } else {
                tab.active = true;
            }
        }

        self.item_categories.get_mut(self.selected_category).unwrap().tick(mouse_pos);

        

        let controlled_player = players.iter().find(|player| {player.owner == Owner::ClientId(*ctx.client_id)});

        let computer_pos = space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap().position();

        if let Some(controlled_player) = controlled_player {

            let player_pos = space.rigid_body_set.get(controlled_player.body.body_handle).unwrap().position();

            let controlled_player_distance: glamx::Vec2 = computer_pos.translation - player_pos.translation;

            if controlled_player_distance.length() > 200. {

                if self.active {
                    self.activated_time = web_time::Instant::now()
                }

                self.active = false;
    
            }  

            else {
                // only set this is we werent already active
                if !self.active {
                    self.activated_time = web_time::Instant::now();
                }

                self.active = true;
            }     

            

            
        }

        let macroquad_pos = rapier_to_macroquad(computer_pos.translation);

        if self.active {
            self.screen_pos = Vec2 {
                x: macroquad_pos.x - 160.,
                y: (macroquad_pos.y - 150.) - 
                    (
                        90. * (self.activated_time.elapsed().as_secs_f32() / 0.15).clamp(0.001, 1.)
                    ),
            };

            self.screen_size = Vec2 {
                x: 320.,
                y: 180. * (self.activated_time.elapsed().as_secs_f32() / 0.15).clamp(0.001, 1.),
            };
        }

        if !self.active {
            self.screen_pos = Vec2 {
                x: macroquad_pos.x - 160.,
                y: (macroquad_pos.y - 150.) -
                    (
                        (1. - ((self.activated_time.elapsed().as_secs_f32() / 0.15).clamp(0.001, 1.))) * 90.
                    ),
            };

            self.screen_size = Vec2 {
                x: 320.,
                y: (1. - ((self.activated_time.elapsed().as_secs_f32() / 0.15).clamp(0.001, 1.))) * 180. ,
            };
        }
        



    }

    pub fn get_mouse_pos(&self, camera_rect: &Rect) -> Vec2 {

        let mouse_pos = mouse_world_pos(camera_rect);
        
        Vec2 {
            x: mouse_pos.x - self.screen_pos.x,
            y: mouse_pos.y - self.screen_pos.y,
        }

        // IN THE FUTURE IF THE RENDER TARGET DOES NOT MATCH THE DESTINATION SIZE THESE COORDS NEED TO MULTIPLIED BY THAT RATIO
    }
}

#[async_trait::async_trait]
impl Drawable for Computer {
    async fn draw(&mut self, draw_context: &DrawContext) {
        self.prop.draw(draw_context).await;

        let prop_pos = draw_context.space.rigid_body_set.get(self.prop.rigid_body_handle).unwrap().position();

        let mut color = BLACK;

        color.a = 0.25;

        let render_target = match &self.render_target {
            Some(render_target) => render_target.clone(),
            None => {
                self.render_target = Some(render_target(320, 180));
                self.render_target.clone().unwrap()
            },
        };

        let camera_rect = Rect::new(0., 0., 320., 180.);

        let mut camera = Camera2D::from_display_rect(camera_rect);

        camera.render_target = Some(render_target.clone());

        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

    
        clear_background(color);        

        let font = draw_context.fonts.get(PathBuf::from("assets/fonts/CutePixel.ttf"));

        //draw_rectangle(0., 0., 20., 20., RED);

        // draw_text_ex("STORE", 0., 20., TextParams {
        //     font: Some(&font),
        //     font_size: 32,
        //     color: WHITE,
        //     ..Default::default()
            
        // });

        for category_tab in &self.category_tabs {
            category_tab.draw(draw_context.fonts);
        }

        let selected_item_category = self.item_categories.get(self.selected_category).unwrap();

        selected_item_category.draw(draw_context.textures, draw_context.prefabs, draw_context.fonts);
        

        // set the camera back
        set_camera(draw_context.default_camera);

        draw_texture_ex(
            &camera.render_target.unwrap().texture, 
            self.screen_pos.x, 
            self.screen_pos.y, 
            WHITE,
            DrawTextureParams {
                dest_size: Some(self.screen_size),
                ..Default::default()
            }
        );
    }

    fn draw_layer(&self) -> u32 {
        1
    }
}