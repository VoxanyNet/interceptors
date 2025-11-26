use std::{fs::{self, File}, process::Command, time::SystemTime};

use macroquad::{color::{DARKGRAY, GRAY, WHITE}, input::{is_mouse_button_released, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle, text::draw_text};

use crate::{ClientTickContext, button::Button, mouse_world_pos, space::Space};

// implementors of this trait can expose their variables to be edited by the context menu 
pub trait EditorContextMenu {
    fn layer(&mut self) -> Option<&mut u32> {
        None
    }

    fn despawn(&mut self) -> Option<&mut bool> {
        None
    }

    fn open_menu(&mut self, position: Vec2) {

        let menu = self.build_menu(position);
        *self.context_menu_data_mut() = Some(menu);
        
        
    }

    fn draw_editor_context_menu(&self) {
        if let Some(menu) = self.context_menu_data() {
            menu.draw();
        }
    }

    fn open_data_editor(&mut self) {
        let json_string = self.data_editor_export().unwrap();

        fs::write("./data_editor_temp.json", json_string).unwrap();

        let menu_data = self.context_menu_data_mut().as_mut().unwrap();

        menu_data.data_editor_last_edit = Some(fs::metadata("./data_editor_temp.json").unwrap().created().unwrap());

        Command::new("code")
            //.arg("--new-window")
            .arg("./data_editor_temp.json")
            .spawn().unwrap();
    }

    fn handle_buttons(&mut self) {
        if let Some(data) = self.context_menu_data_mut() {
            for entry in data.entries.clone() {
                if entry.button.released {
                    match entry.field_type {
                        EntryType::IncreaseLayer => *self.layer().unwrap() += 1,
                        EntryType::DecreaseLayer => * self.layer().unwrap() = self.layer().unwrap().saturating_sub(1),
                        EntryType::DataEditor => self.open_data_editor(),
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
    
    fn apply_data_editor_updates(&mut self) {
        // someday i will come back to this code and look on in horror
        match self.context_menu_data_mut() {
            Some(data) => {

                match data.data_editor_last_edit {

                    Some(last_edit) => {

                        if fs::metadata("./data_editor_temp.json").unwrap().modified().unwrap() > last_edit {

                            println!("object updated!");

                            // need to preserve the context menu data lololol!
                            let old_editor_context_menu_data = self.context_menu_data().as_ref().unwrap().clone();

                            self.data_editor_import(fs::read_to_string("./data_editor_temp.json").unwrap());

                            *self.context_menu_data_mut() = Some(old_editor_context_menu_data);

                            self.context_menu_data_mut().as_mut().unwrap().data_editor_last_edit = Some(fs::metadata("./data_editor_temp.json").unwrap().modified().unwrap());
                        }
                    },
                    // object is not open in editor
                    None => {},
                }
            },
            // object doesnt have data for some reason
            None => {},
        }
    }
    fn update_menu(&mut self, space: &Space, camera_rect: &Rect) {

        if is_mouse_button_released(macroquad::input::MouseButton::Right) && self.object_bounding_box(Some(space)).contains(mouse_world_pos(camera_rect)) {
            println!("open");
            self.open_menu(mouse_position().into());
        }

        if (is_mouse_button_released(macroquad::input::MouseButton::Left) || is_mouse_button_released(macroquad::input::MouseButton::Right)) && !self.object_bounding_box(Some(space)).contains(mouse_world_pos(camera_rect)) {
            println!("close");
            self.close_menu();
        }

        self.apply_data_editor_updates();

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

    fn data_editor_export(&self) -> Option<String> {
        None
    }

    fn data_editor_import(&mut self, json: String) {
        // just do nothing by default
    }

    fn object_bounding_box(&self, space: Option<&Space>) -> Rect;

    fn build_menu(&mut self, position: Vec2) -> EditorContextMenuData {
        let mut entries: Vec<MenuEntry> = vec![];

        let mut entry_index = 0;

        if self.data_editor_export().is_some() {
            entries.push(
                MenuEntry {
                    button: Button::new(
                        Rect::new(position.x, position.y + (20. * entry_index as f32), 150., 20.), None
                    ),
                    field_type: EntryType::DataEditor,
                }
            );

            entry_index += 1;
        }
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
            data_editor_last_edit: None,
            
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
    entries: Vec<MenuEntry>,
    pub data_editor_last_edit: Option<SystemTime>, // this also indicates that the object is open in the data editor at all

}

impl EditorContextMenuData {
    pub fn draw(&self) {

        dbg!("drawing");

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
    DecreaseLayer,
    DataEditor
}
