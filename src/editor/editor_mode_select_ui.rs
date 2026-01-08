use std::{path::PathBuf, str::FromStr};

use interceptors_lib::{button::Button, texture_loader::TextureLoader};
use macroquad::{color::WHITE, input::mouse_position, math::{Rect, Vec2}, shapes::draw_rectangle, texture::{DrawTextureParams, draw_texture_ex}, window::screen_width};

use crate::{editor_input_context::EditorInputContext, editor_ui_tick_context::EditorUITickContext};

pub struct EditorModeSelectUI {
   select_mode_toggle: Button,
   prefab_mode_toggle: Button,
   simulate_space_toggle: Button

}

impl EditorModeSelectUI {
    pub fn new() -> Self {

        let select_mode_toggle = Button::new(Rect::new(screen_width() - 32., 32., 32., 32.), None);
        let prefab_mode_toggle = Button::new(Rect::new(screen_width() - 32., 64., 32., 32.), None);
        
        let simulate_space_toggle = Button::new(Rect::new(screen_width() - 32., 128., 32., 32.), None);
        
        Self {
            select_mode_toggle,
            prefab_mode_toggle,
            simulate_space_toggle
        }
    }
    
    pub fn hovered(&self) -> bool {
        if self.select_mode_toggle.hovered {
            true
        } else if self.prefab_mode_toggle.hovered {
            true
        } else if self.simulate_space_toggle.hovered {
            true
        }
         else {
            false
        }
    }

    pub fn update_buttons(&mut self) {
        let mouse_pos: Vec2 = mouse_position().into();

        self.select_mode_toggle.update(mouse_pos);
        self.prefab_mode_toggle.update(mouse_pos);

        self.simulate_space_toggle.update(mouse_pos);
    }

    pub fn handle_buttons(&mut self, ctx: &mut EditorUITickContext) {

        if ctx.input_context != EditorInputContext::EditorModeMenu {
            return;
        }

        if self.select_mode_toggle.released {
            
            *ctx.selected_mode = 0
        }

        if self.prefab_mode_toggle.released {
            *ctx.selected_mode = 1;
        }

        if self.simulate_space_toggle.released {
            *ctx.simulate_space = !*ctx.simulate_space;
        }


    }

    pub fn update(&mut self, ctx: &mut EditorUITickContext) {
        
        self.update_buttons();
        self.reposition_elements();
        self.handle_buttons(ctx);

    }


    pub fn reposition_elements(&mut self) {
        self.prefab_mode_toggle.rect.x = screen_width() - self.prefab_mode_toggle.rect.w;
        
        self.select_mode_toggle.rect.x = screen_width() - self.prefab_mode_toggle.rect.w;

        self.simulate_space_toggle.rect.x = screen_width() - self.prefab_mode_toggle.rect.w;
    }

    pub fn draw(&self, textures: &TextureLoader) {

        self.draw_button(textures, &self.select_mode_toggle, "assets/ui/cursor.png");
        self.draw_button(textures, &self.prefab_mode_toggle, "assets/ui/spawner.png");
        self.draw_button(textures, &self.simulate_space_toggle, "assets/ui/simulate_space.png");

    }

    pub fn draw_button(&self, textures: &TextureLoader, button: &Button, image_path: &str) {
        let mut button_background_color = WHITE;
        button_background_color.a = 0.5;

        // background
        draw_rectangle(button.rect.x, button.rect.y, button.rect.w, button.rect.h, button_background_color);

        // image
        draw_texture_ex(
            textures.get(&PathBuf::from_str(image_path).unwrap()), 
            button.rect.x, 
            button.rect.y, 
            WHITE, 
            DrawTextureParams {
                dest_size: Vec2::new(button.rect.w, button.rect.h).into(),
                ..Default::default()
            }
        );

    }
}