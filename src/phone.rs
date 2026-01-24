use std::path::PathBuf;

use macroquad::{color::WHITE, math::{Rect, Vec2}, texture::{draw_texture_ex, DrawTextureParams}, window::{screen_height, screen_width}};

use crate::texture_loader::ClientTextureLoader;

enum PhoneAnimationState {
    Opening,
    Closing,
    Static
}
pub struct Phone {
    frame: i32,
    last_frame: web_time::Instant,
    animation_state: PhoneAnimationState,
    open: bool
}

impl Phone {

    pub fn close(&mut self) {
        
        self.animation_state = PhoneAnimationState::Closing;
    }

    pub fn open(&mut self) {
        self.animation_state = PhoneAnimationState::Opening;
    }   

    pub fn toggle(&mut self) {
        match self.open {
            true => self.close(),
            false => self.open(),
        }
    }

    pub fn update_animation(&mut self) {

        // this should be less hard coded

        if self.last_frame.elapsed().as_secs_f32() > 0.05 {

            let frame_change = match self.animation_state {
                PhoneAnimationState::Opening => {
                    -1
                },
                PhoneAnimationState::Closing => {
                    1
                },
                PhoneAnimationState::Static => {
                    0
                },
            };

            self.frame += frame_change;

            // this logic is really dumb
            if self.frame == 1 {
                self.open = true;

                self.animation_state = PhoneAnimationState::Static

            } else {
                self.open = false;
            }
            
            if self.frame == 4 {
                self.animation_state = PhoneAnimationState::Static
            }

            self.last_frame = web_time::Instant::now();
        }
    }

    pub fn new() -> Self {
        Self {
            frame: 4,
            last_frame: web_time::Instant::now(),
            animation_state: PhoneAnimationState::Static,
            open: false
        }
    }

    pub fn tick(&mut self) {
        self.update_animation();
    }
    pub fn draw(&self, textures: &ClientTextureLoader, _camera_rect: &Rect) {
        let texture = textures.get(&PathBuf::from(format!("assets/phone/{:?}.png", self.frame)));

        let mut params = DrawTextureParams::default();

        params.dest_size = Some(
            Vec2::new(screen_width(), screen_height())
        );

        draw_texture_ex(
            texture, 
            0., 
            0., 
            WHITE,
            params
        );

    }
}