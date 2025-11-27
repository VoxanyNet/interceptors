use std::{path::{Path, PathBuf}, str::FromStr};

use interceptors_lib::{button::Button, drawable::Drawable, font_loader::FontLoader};
use macroquad::{color::{GRAY, LIGHTGRAY, WHITE}, input::mouse_position, math::{Rect, Vec2}, shapes::draw_rectangle, text::{TextParams, draw_text, draw_text_ex}, window::{screen_height, screen_width}};

struct LayerToggle {
    layer: u32,
    button: Button,
    enabled: bool
}

impl LayerToggle {
    pub fn draw(&self, fonts: &FontLoader) {

        let color = match self.enabled {
            true => LIGHTGRAY,
            false => GRAY,
        };

        draw_rectangle(self.button.rect.x, self.button.rect.y, self.button.rect.w, self.button.rect.h, color);

        draw_text_ex(
            &format!("{}", self.layer), 
            self.button.rect.x + 16., 
            self.button.rect.y + 16., 
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
    previous_update_layers: Vec<u32> // we use this to check if we need to rebuild the ui
}

impl LayerToggleUI {


    pub fn new() -> Self {

        Self {
            toggles: Vec::new(),
            position: Vec2::ZERO,
            previous_update_layers: Vec::new()
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
        for toggle in &mut self.toggles {
            toggle.button.update(mouse_position().into());

            if toggle.button.released {
                toggle.enabled = !toggle.enabled;
            }

        }
    }

    pub fn get_disabled_layers(&self) -> Vec<u32> {

        let mut disabled_layers = Vec::new();
        for toggle in &self.toggles {
            if !toggle.enabled {
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

    pub fn draw(&self, fonts: &FontLoader) {
        for toggle in &self.toggles {
            toggle.draw(fonts);
        }
    }

    fn reposition_elements(&mut self) {
        self.position.x = screen_width() - 32.;
        self.position.y = screen_height() - (32. * self.toggles.len() as f32);

        for (index, toggle) in self.toggles.iter_mut().enumerate() {
            toggle.button.rect.x = self.position.x;
            toggle.button.rect.y = (self.position.y) + (index as f32 * 32.);
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
                    button: Button::new(
                        Rect::new(
                            self.position.x, 
                            (self.position.y) + (index as f32 * 32.), 
                            32., 
                            32.
                        ), 
                        None
                    ),
                    enabled: true
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