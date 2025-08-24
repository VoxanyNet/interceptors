use macroquad::{input::{is_mouse_button_down, is_mouse_button_released, mouse_position}, math::{Rect, Vec2}};

use crate::mouse_world_pos;

#[derive(Debug, Clone, Copy)]
pub struct Button {
    pub hovered: bool,
    pub down: bool,
    pub released: bool,
    pub rect: Rect
}

impl Button {

    pub fn new(rect: Rect) -> Self {
        Self {
            hovered: false,
            down: false,
            released: false,
            rect,
        }
    }
    pub fn update(&mut self, camera_rect: &Rect, offset: Vec2) {

        // this makes me ANGRY!!!!!
        let offset_rect = Rect {
            x: camera_rect.x + offset.x,
            y: camera_rect.y + offset.y,
            w: camera_rect.w,
            h: camera_rect.h,
        };

        if offset_rect.contains(
            mouse_world_pos(camera_rect)
        ) {
            self.hovered = true;

            if is_mouse_button_down(macroquad::input::MouseButton::Left) {
                self.down = true;
            } else {
                self.down = false;
            }

            if is_mouse_button_released(macroquad::input::MouseButton::Left) {
                self.released = true;
            } else {
                self.released = false;
            }
        } else {
            self.hovered = false;
            self.down = false;
            self.released = false;
        }
    }
}