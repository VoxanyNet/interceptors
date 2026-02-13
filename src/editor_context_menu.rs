use std::{env::temp_dir, fs::{self}, path::PathBuf, process::Command, time::SystemTime};

use macroquad::{color::{DARKGRAY, GRAY, WHITE}, input::{KeyCode, is_key_down, is_key_released, is_mouse_button_released, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle, text::draw_text};
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::{button::Button, mouse_world_pos, space::Space, texture_loader::ClientTextureLoader, uuid_string};

pub struct DataEditorContext<'a> {
    pub space: &'a mut Space,
    pub textures: &'a ClientTextureLoader
}
// implementors of this trait can expose their variables to be edited by the context menu 
pub trait EditorContextMenu {
    fn layer(&mut self) -> Option<&mut u32> {
        None
    }

    fn rigid_body(&mut self) -> Option<RigidBodyHandle> {
        None
    }

    fn despawn(&mut self) -> Option<&mut bool> {
        None
    }

    fn collider(&mut self) -> Option<ColliderHandle> {
        None
    }

    fn open_menu(&mut self, position: Vec2, ctx: &DataEditorContext) {

        let menu = self.build_menu(position, ctx);
        *self.context_menu_data_mut() = Some(menu);
        
        
    }

    fn draw_editor_context_menu(&self) {

        if let Some(menu) = self.context_menu_data() {
            menu.draw();
        }
    }

    // not sure if this should be in here
    fn handle_hotkeys(&mut self, selected: bool, space: &mut Space) {
        
        if !selected {
            return;
        }

        if let Some(despawn) = self.despawn() {
            if is_key_released(macroquad::input::KeyCode::Delete)  {
                *despawn = true
            }
        }
        
        if let Some(collider_handle) = self.collider() && let Some(rigid_body_handle) = self.rigid_body() {
            if is_key_down(KeyCode::LeftShift) {

                let mut x_delta = 0.;
                let mut y_delta = 0.;
                if is_key_released(KeyCode::Left) {
                    x_delta = -10.;
                }

                if is_key_released(KeyCode::Right) {
                    x_delta = 10.;
                }

                if is_key_released(KeyCode::Up) {
                    y_delta = 10.;
                }

                if is_key_released(KeyCode::Down) {
                    y_delta = -10.;
                }


                let collider = space.collider_set.get_mut(collider_handle).unwrap();
                let rigid_body = space.rigid_body_set.get_mut(rigid_body_handle).unwrap();

                let shape = collider.shape_mut().as_cuboid_mut().unwrap();
                shape.half_extents.x += x_delta;
                shape.half_extents.y += y_delta;

                let mut new_pos = rigid_body.position().clone();
                new_pos.translation.x += x_delta;
                new_pos.translation.y += y_delta;

                rigid_body.set_position(
                    new_pos, 
                    true
                );
            }
        }
        
    }

    fn open_data_editor(&mut self, ctx: &DataEditorContext) {
        let json_string = self.data_editor_export(ctx).unwrap();

        let menu_data = self.context_menu_data_mut().as_mut().unwrap();


        fs::write(&menu_data.data_editor_file_path, json_string).unwrap();

        
        menu_data.data_editor_last_edit = Some(fs::metadata(&menu_data.data_editor_file_path).unwrap().created().unwrap());

        #[cfg(target_os = "linux")]
        Command::new("codium")
            .arg(&menu_data.data_editor_file_path)
            .spawn().unwrap();

        #[cfg(target_os = "windows")]
        Command::new("powershell")
            .arg("-Command")
            .arg(format!("codium \"{}\"", &menu_data.data_editor_file_path.to_string_lossy().to_string()))
            .spawn().unwrap();
    }

    fn handle_buttons(&mut self, ctx: DataEditorContext) {
        if let Some(data) = self.context_menu_data_mut() {
            for entry in data.entries.clone() {
                if entry.button.released {
                    match entry.field_type {
                        EntryType::IncreaseLayer => *self.layer().unwrap() += 1,
                        EntryType::DecreaseLayer => * self.layer().unwrap() = self.layer().unwrap().saturating_sub(1),
                        EntryType::DataEditor => self.open_data_editor(&ctx),
                        EntryType::Despawn => *self.despawn().unwrap() = true
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
    
    fn apply_data_editor_updates(&mut self, ctx: &mut DataEditorContext) {
        // someday i will come back to this code and look on in horror
        match self.context_menu_data_mut() {
            Some(data) => {
                match data.data_editor_last_edit {
                    Some(last_edit) => {

                        if fs::metadata(&data.data_editor_file_path).unwrap().modified().unwrap() > last_edit {
                            // need to preserve the context menu data lololol!
                            let old_editor_context_menu_data = self.context_menu_data().as_ref().unwrap().clone();
                            self.data_editor_import(fs::read_to_string(&old_editor_context_menu_data.data_editor_file_path).unwrap(), ctx);
                            *self.context_menu_data_mut() = Some(old_editor_context_menu_data.clone());
                            self.context_menu_data_mut().as_mut().unwrap().data_editor_last_edit = Some(fs::metadata(old_editor_context_menu_data.data_editor_file_path).unwrap().modified().unwrap());
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

    fn update_menu(
        &mut self, 
        space: &mut Space, 
        camera_rect: &Rect, 
        selected: bool,
        textures: &ClientTextureLoader 
    ) {
        
        // not sure if this is where this should be. maybe i should rename the trait. the hotkeys access the same data as the menu so this is a good spot to put it
        self.handle_hotkeys(selected, space);
        
        if (is_mouse_button_released(macroquad::input::MouseButton::Left) || is_mouse_button_released(macroquad::input::MouseButton::Right)) && !self.contains_point(mouse_position().into()) {
            self.close_menu();
        }
        if selected && is_mouse_button_released(macroquad::input::MouseButton::Right) && self.object_bounding_box(Some(space)).contains(mouse_world_pos(camera_rect)) {
            self.open_menu(mouse_position().into(), &DataEditorContext { space, textures });
        }

        self.apply_data_editor_updates(&mut DataEditorContext { space, textures });
        self.update_buttons();
        self.handle_buttons(
            DataEditorContext {
                space,
                textures,
                
            }
        );
    }

    

    fn menu_rect(&self) -> Rect {

        let mut rect = Rect::default();
        if let Some(data) = self.context_menu_data() {
            for entry in &data.entries {
                rect = rect.combine_with(entry.button.rect)
            }
        }

        rect
    }

    fn contains_point(&self, point: Vec2) -> bool {
        if let Some(data) = self.context_menu_data() {
            for entry in &data.entries {
                if entry.button.rect.contains(point) {
                    return true
                }
            }
        }

        false
    }

    fn data_editor_export(&self, _ctx: &DataEditorContext) -> Option<String> {
        None
    }

    fn data_editor_import(&mut self, _json: String, _ctx: &mut DataEditorContext) {
        // just do nothing by default
    }

    fn object_bounding_box(&self, space: Option<&Space>) -> Rect;

    fn build_menu(&mut self, position: Vec2, ctx: &DataEditorContext) -> EditorContextMenuData {
        let mut entries: Vec<MenuEntry> = vec![];

        let mut entry_index = 0;

        if self.despawn().is_some() {
            entries.push(
                MenuEntry {
                    button: Button::new(
                        Rect::new(position.x, position.y + (20. * entry_index as f32), 150., 20.),
                        None
                    ),
                    field_type: EntryType::Despawn
                }
            );

            entry_index += 1;
        }
        if self.data_editor_export(ctx).is_some() {
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
            data_editor_file_path: temp_dir().join("editor_".to_string() + &uuid_string() + ".json").into()
        }
    }
    fn context_menu_data_mut(&mut self) -> &mut Option<EditorContextMenuData>;
    fn context_menu_data(&self) -> &Option<EditorContextMenuData>;

    fn close_menu(&mut self) {
        
        *self.context_menu_data_mut() = None;
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EditorContextMenuData {
    entries: Vec<MenuEntry>,
    pub data_editor_last_edit: Option<SystemTime>, // this also indicates that the object is open in the data editor at all
    pub data_editor_file_path: PathBuf

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

#[derive(Clone, PartialEq, Debug)]
pub struct MenuEntry {
    button: Button,
    field_type: EntryType
}

#[derive(strum::Display, Clone, PartialEq, Debug)]
pub enum EntryType {
    Despawn,
    IncreaseLayer,
    DecreaseLayer,
    DataEditor
}
