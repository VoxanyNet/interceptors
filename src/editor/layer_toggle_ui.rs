use std::{path::PathBuf, str::FromStr};

use interceptors_lib::{button::Button, drawable::Drawable, font_loader::FontLoader, texture_loader::TextureLoader};
use macroquad::{color::{GRAY, LIGHTGRAY, WHITE}, input::mouse_position, math::{Rect, Vec2}, shapes::draw_rectangle, text::{TextParams, draw_text_ex}, texture::{DrawTextureParams, draw_texture_ex}, window::{screen_height, screen_width}};

struct LayerToggle {
    layer: u32,
    pub active_toggle: Button, 
    visibility_toggle: Button, 
    lock_toggle: Button,
    visible: bool
}

impl LayerToggle {
    pub fn draw(
        &self, 
        fonts: &FontLoader, 
        textures: &TextureLoader,
        active: bool // the layer toggle's 'active' state is managed from the outside
    ) { 

        let color = match active {
            true => GRAY,
            false => LIGHTGRAY,
        };

        draw_rectangle(self.active_toggle.rect.x, self.active_toggle.rect.y, self.active_toggle.rect.w, self.active_toggle.rect.h, color);
        
        draw_rectangle(self.visibility_toggle.rect.x, self.visibility_toggle.rect.y, self.visibility_toggle.rect.w, self.visibility_toggle.rect.h, GRAY);

        draw_rectangle(self.lock_toggle.rect.x, self.lock_toggle.rect.y, self.lock_toggle.rect.w, self.lock_toggle.rect.h, GRAY);

        if self.visible {
            draw_texture_ex(
                textures.get(&PathBuf::from_str("assets/ui/eye.png").unwrap()), 
                self.visibility_toggle.rect.x, 
                self.visibility_toggle.rect.y, 
                WHITE, 
                DrawTextureParams {
                    dest_size: Some(Vec2::new(32., 32.)),
                    ..Default::default()
                }
            );
        }

        draw_text_ex(
            &format!("{}", self.layer), 
            self.active_toggle.rect.x + 16., 
            self.active_toggle.rect.y + 16., 
            TextParams {
                font: Some(&fonts.get(PathBuf::from_str("assets/fonts/CutePixel.ttf").unwrap())),
                font_size: 20,
                color: WHITE,
                ..Default::default()
            }
        );
    }
}
pub struct LayerToggleUI {
    position: Vec2,
    toggles: Vec<LayerToggle>,
    pub active_layer: u32,
    previous_update_layers: Vec<u32> // we use this to check if we need to rebuild the ui
}

impl LayerToggleUI {


    pub fn new() -> Self {

        Self {
            toggles: Vec::new(),
            position: Vec2::ZERO,
            previous_update_layers: Vec::new(),
            active_layer: 0
        }

    }

    fn attempt_rebuild(&mut self, drawable_objects: Vec<&dyn Drawable>) {
        let object_layers = Self::get_object_layers(drawable_objects.clone());

        if self.previous_update_layers != Self::get_object_layers(drawable_objects.clone()) {

            self.rebuild(drawable_objects.clone());
        }

        self.previous_update_layers = object_layers;
    }

    fn update_toggles(&mut self) {
        for (index, toggle) in self.toggles.iter_mut().enumerate().rev() {

            toggle.active_toggle.update(mouse_position().into());
            toggle.visibility_toggle.update(mouse_position().into());

            if toggle.visibility_toggle.released {
                toggle.visible = !toggle.visible;
            }

            if toggle.active_toggle.released {
                self.active_layer = index as u32
            }

        }
    }

    pub fn get_disabled_layers(&self) -> Vec<u32> {

        let mut disabled_layers = Vec::new();
        for toggle in &self.toggles {
            if !toggle.visible {
                disabled_layers.push(toggle.layer);
            }
        }

        disabled_layers
    }

    pub fn update(&mut self, drawable_objects: Vec<&dyn Drawable>) {
    
        self.attempt_rebuild(drawable_objects);
        self.reposition_elements();
        self.update_toggles();


    }

    pub fn draw(&self, fonts: &FontLoader, textures: &TextureLoader) {
        for (index, toggle) in self.toggles.iter().enumerate() {
            
            toggle.draw(fonts, textures, self.active_layer == index as u32);
        }



        // draw_texture_ex(
        //     textures.get(&PathBuf::from_str("assets/ui/arrow_right.png").unwrap()), 
        //     screen_width() - 64., 
        //     screen_height() - (32. * self.active_layer as f32) - 16., 
        //     WHITE
        // );
    }

    fn reposition_elements(&mut self) {
        self.position.x = screen_width() - 32.;
        self.position.y = screen_height() - (32. * self.toggles.len() as f32);

        for (index, toggle) in self.toggles.iter_mut().enumerate() {
            toggle.active_toggle.rect.x = self.position.x;
            toggle.active_toggle.rect.y = (self.position.y) + (index as f32 * 32.);
            toggle.visibility_toggle.rect.x = self.position.x - 32.;
            toggle.visibility_toggle.rect.y = (self.position.y) + (index as f32 * 32.);
        }
    }

    fn rebuild(&mut self, drawable_objects: Vec<&dyn Drawable>) {

        self.toggles.clear();

        let mut layers = Self::get_object_layers(drawable_objects);
        layers.sort();

        for (index, layer) in layers.iter().enumerate() {
            self.toggles.push(
                LayerToggle {
                    layer: *layer,
                    active_toggle: Button::new(
                        Rect::new(
                            self.position.x, 
                            (self.position.y) + (index as f32 * 32.), 
                            32., 
                            32.
                        ), 
                        None
                    ),
                    visibility_toggle: Button::new(
                        Rect::new(
                            self.position.x - 32., 
                            (self.position.y) + (index as f32 * 32.), 
                            32., 
                            32.
                        ), 
                        None
                    ),
                    lock_toggle: Button::new(
                        Rect::new(
                            self.position.x - 64., 
                            (self.position.y) + (index as f32 * 32.), 
                            32., 
                            32.
                        ), 
                        None
                    ),
                    visible: true
                }
            );
        }


    }   

    fn get_object_layers(drawable_objects: Vec<&dyn Drawable>) -> Vec<u32> {
        // optimization is for losers
        
        let mut object_layers: Vec<u32> = vec![];

        for object in drawable_objects {
            if !object_layers.contains(&object.draw_layer()) {
                object_layers.push(object.draw_layer());
            }
        }

        object_layers
    }
}