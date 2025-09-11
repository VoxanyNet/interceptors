use macroquad::{input::{is_mouse_button_down, is_mouse_button_released}, math::{Rect, Vec2}};


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
    pub fn update(&mut self, mouse_pos: Vec2) {

        
        if self.rect.contains(
            mouse_pos
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