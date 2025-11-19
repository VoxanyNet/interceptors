use macroquad::{color::{DARKGRAY, GRAY, WHITE}, input::{is_mouse_button_released, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle, text::draw_text};

use crate::button::Button;

// implementors of this trait can expose their variables to be edited by the context menu 
pub trait EditorContextMenu {
    fn layer(&mut self) -> Option<&mut u32> {
        None
    }

    fn open_menu(&mut self, position: Vec2) {

        let menu = self.build_menu(position);
        let context_menu = self.context_menu_data_mut();
        *context_menu = Some(menu);
        
    }

    fn draw_editor_context_menu(&self) {
        if let Some(menu) = self.context_menu_data() {
            menu.draw();
        }
    }

    fn handle_buttons(&mut self) {
        if let Some(data) = self.context_menu_data_mut() {
            for entry in data.entries.clone() {
                if entry.button.released {
                    match entry.field_type {
                        EntryType::IncreaseLayer => *self.layer().unwrap() += 1,
                        EntryType::DecreaseLayer => *self.layer().unwrap() -= 1,
                    }
                }
            }
        };
    }

    fn update_buttons(&mut self) {
        if let Some(data) = self.context_menu_data_mut() {
            for entry in &mut data.entries {
                entry.button.update(mouse_position().into());
            }
        }
    }

    fn update_menu(&mut self) {

        if is_mouse_button_released(macroquad::input::MouseButton::Right) && self.object_bounding_box().contains(mouse_position().into()) {
            println!("open");
            self.open_menu(mouse_position().into());
        }

        if (is_mouse_button_released(macroquad::input::MouseButton::Left) || is_mouse_button_released(macroquad::input::MouseButton::Right)) && !self.object_bounding_box().contains(mouse_position().into()) {
            println!("close");
            self.close_menu();
        }

        self.update_buttons();
        self.handle_buttons();




    }

    fn menu_rect(&mut self) -> Rect {

        let mut rect = Rect::default();
        if let Some(data) = self.context_menu_data_mut() {
            for entry in &data.entries {
                rect = rect.combine_with(entry.button.rect)
            }
        }

        rect
    }

    fn object_bounding_box(&self) -> Rect;

    fn build_menu(&mut self, position: Vec2) -> EditorContextMenuData {
        let mut entries: Vec<MenuEntry> = vec![];

        let mut entry_index = 0;

        if self.layer().is_some() {

            entries.push(
                MenuEntry {
                    button: Button::new(
                        Rect::new(position.x, position.y + (20. * entry_index as f32), 150., 20.), None
                    ),
                    field_type: EntryType::IncreaseLayer,
                }
            );

            entry_index += 1;

            entries.push(
                MenuEntry {
                    button: Button::new(
                        Rect::new(position.x, position.y + (20. * entry_index as f32), 150., 20.), None
                    ),
                    field_type: EntryType::DecreaseLayer,
                }
            );

            entry_index += 1;
        }

        EditorContextMenuData {
            entries,
        }
    }
    fn context_menu_data_mut(&mut self) -> &mut Option<EditorContextMenuData>;
    fn context_menu_data(&self) -> &Option<EditorContextMenuData>;

    fn close_menu(&mut self) {
        *self.context_menu_data_mut() = None;
    }
}

#[derive(Clone, PartialEq)]
pub struct EditorContextMenuData {
    entries: Vec<MenuEntry>
}

impl EditorContextMenuData {
    pub fn draw(&self) {
        for entry in &self.entries {

            let rect = entry.button.rect;

            let color = match entry.button.hovered {
                true => DARKGRAY,
                false => GRAY,
            };

            draw_rectangle(rect.x, rect.y,rect.w, rect.h, color);
            draw_text(&entry.field_type.to_string(), rect.x, rect.y + 12., 20., WHITE);

        }
    }
}

#[derive(Clone, PartialEq)]
pub struct MenuEntry {
    button: Button,
    field_type: EntryType
}

#[derive(strum::Display, Clone, PartialEq)]
pub enum EntryType {
    IncreaseLayer,
    DecreaseLayer
}
