use std::{fs::read_to_string, path::PathBuf, str::FromStr};

use interceptors_lib::{area::Area, background::{Background, BackgroundSave}, button::Button, decoration::{Decoration, DecorationSave}, drawable::{DrawContext, Drawable}, prop::{Prop, PropSave}, space::Space, texture_loader::TextureLoader, tile::{Tile, TileSave}};
use macroquad::{color::{GREEN, LIGHTGRAY, WHITE}, input::{MouseButton, is_mouse_button_released, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle_lines, text::draw_text};
use nalgebra::{vector, Vector2};
use strum::IntoEnumIterator;

use crate::{editor_input_context::EditorInputContext, list_dir_entries, spawner_category::SpawnerCategory, spawner_menu::SpawnerMenu};


pub struct Spawner {
    menu: SpawnerMenu,
    selected_prefab: i32, // vector index
    selected_category: SpawnerCategory,
    change: bool,
    selected_prefab_json: String, // the path to the currently selected prefab
    prefab_buttons: Vec<(Button, i32)>, // button -> prefab index
    category_buttons: Vec<(Button, SpawnerCategory)>,
    preview_space: Space
}

impl Spawner {
    pub async fn new() -> Self {
        let spawner_menu = SpawnerMenu::new();
        
        let mut category_x = 0.;
        
        // genius
        let category_buttons: Vec<(Button, SpawnerCategory)> = SpawnerCategory::iter()
            .map(
                |category| 
                {
                    
                    let category_text_width = category.to_string().len() as f32 * 12.; 
                    
                    let button = (Button::new(Rect::new(category_x, 0., category_text_width, 20.), None), category);

                    category_x += category.to_string().len() as f32 * 12.;

                    button
                } 
            )
            .collect();
        
            

        let mut spawner = Self {
            menu: spawner_menu,
            selected_prefab: 0,
            selected_category: SpawnerCategory::Prop,
            change: true,
            selected_prefab_json: String::new(),
            prefab_buttons: Vec::new(),
            category_buttons,
            preview_space: Space::new()
        };

        spawner.load_prefab();

        spawner.rebuild_buttons();

        spawner


    }

    pub fn rebuild_buttons(&mut self) {
        self.prefab_buttons.clear();

        for (index, prefab_path) in self.menu.prefabs.get(&self.selected_category).unwrap().iter().enumerate() {

            let y = ((index) * 20) as f32 + 20.;

            let width = prefab_path.len() as f32 * 9.;

            let button = Button::new(
                Rect::new(0., y, width, 20.), 
                None
            );

            self.prefab_buttons.push((button, index as i32));
        }
    }

    pub fn hovered(&self) -> bool {
        for (button, _) in &self.prefab_buttons {
            if button.hovered {
                return true
            }
        }

        for (button, _) in &self.category_buttons {
            if button.hovered {
                return true;
            }
        }

        return false
    }

    pub fn update_buttons(&mut self) {


        let mouse_pos = mouse_position();
        let mouse_pos = Vec2::new(mouse_pos.0, mouse_pos.1);

        for (button, _) in &mut self.prefab_buttons {
            button.update(mouse_pos);
        }

        for (button, _) in &mut self.category_buttons {
            button.update(mouse_pos);
        }
    }

    pub fn handle_buttons(&mut self, editor_input_context: EditorInputContext) {

        if editor_input_context != EditorInputContext::SpawnerMenu {
            return;
        }
        for (button, index) in &self.prefab_buttons {
            if button.released {
                self.selected_prefab = *index;
                self.change = true;
            }
        }

        for (button, spawner_category) in &self.category_buttons {
            if button.released {
                self.selected_category = *spawner_category;

                self.change = true;
            }
        }
    }

    

    pub fn delete_selected_object(&mut self) {

    }

    pub fn tick(
        &mut self, 
        area: &mut Area, 
        camera_rect: &Rect, 
        cursor: Vec2, 
        rapier_cursor: Vec2,
        input_context: EditorInputContext
    ) {


        self.change = false;


        self.update_buttons();
        self.handle_buttons(input_context);

        if self.change {
            self.rebuild_buttons();
        }


        let category = &self.selected_category;

        if self.selected_prefab > self.menu.prefabs.get(category).unwrap().len() as i32 - 1  {
            self.selected_prefab = 0
        }

        if self.selected_prefab < 0 {
            self.selected_prefab = self.menu.prefabs.get(category).unwrap().len() as i32 - 1;
        }

        self.load_prefab();


        if is_mouse_button_released(MouseButton::Left) && input_context == EditorInputContext::World {
            self.spawn(area, camera_rect, cursor, rapier_cursor);
        }

        

    }

    pub fn current_prefab(&self) -> String {
        let prefabs = self.menu.prefabs.get(&self.selected_category).unwrap();

        prefabs.get(self.selected_prefab as usize).unwrap().clone()
    }

    pub fn load_prefab(&mut self) {
        if self.change {
            let selected_prefab_path = self.menu.prefabs.get(&self.selected_category).unwrap().get(self.selected_prefab as usize).unwrap().clone();


            let path = PathBuf::from_str(&selected_prefab_path).unwrap();

            // enter into directory
            if path.is_dir() {
                *self.menu.prefabs.get_mut(&self.selected_category).unwrap() = list_dir_entries(path.clone()).unwrap();
                
                // add the parent directory so you can go back up
                self.menu.prefabs.get_mut(&self.selected_category).unwrap().insert(0, path.parent().unwrap().to_string_lossy().to_string());

                // load the first entry in the directory
                self.selected_prefab = 1;
                let selected_prefab_path = self.menu.prefabs.get(&self.selected_category).unwrap().get(self.selected_prefab as usize).unwrap().clone();
                self.selected_prefab_json = read_to_string(selected_prefab_path).unwrap();
                
                self.rebuild_buttons();

                
            } else {
                self.selected_prefab_json = read_to_string(selected_prefab_path).unwrap();
            }

            
        }   
    }

    /// Get the bounding box for cursor snapping in macroquad coords.
    /// Returns None if snapping is not supported
    pub fn get_snapping_bounding_box(&self, cursor: Vec2) -> Option<Rect> {
        match self.selected_category {
            SpawnerCategory::Decoration => {
                let decoration_save: DecorationSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut decoration: Decoration = Decoration::from_save(decoration_save);

                decoration.pos = cursor;

                return Some(Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y))
                
            },
            SpawnerCategory::Background => {
                return None
            },
            SpawnerCategory::Prop => {
                None
            },
            SpawnerCategory::Tile => {
                return  Some(Rect::new(cursor.x, cursor.y, 50., 50.));
            }
        }
    }

    pub async fn draw_preview_spawn(&mut self, draw_context: &DrawContext<'_>, cursor: Vec2, rapier_cursor: Vec2) {

        

        match self.selected_category {
            
            SpawnerCategory::Decoration => {
                let decoration_save: DecorationSave = serde_json::from_str(&self.selected_prefab_json).unwrap();
                let mut decoration: Decoration = Decoration::from_save(decoration_save);
                decoration.pos = cursor;
                decoration.draw(&draw_context).await  
            },
            SpawnerCategory::Background => {
                let background_save: BackgroundSave = serde_json::from_str(&self.selected_prefab_json).unwrap();
                let mut background = Background::from_save(background_save);
                background.pos = cursor;
                background.draw(&draw_context).await
            },

            SpawnerCategory::Prop => {
                let generic_physics_prop_save: PropSave = serde_json::from_str(&self.selected_prefab_json).unwrap();
                let mut generic_physics_prop = Prop::from_save(generic_physics_prop_save.clone(), &mut self.preview_space);
                generic_physics_prop.set_pos(vector![rapier_cursor.x + generic_physics_prop_save.size.x / 2., rapier_cursor.y - generic_physics_prop_save.size.y / 2.].into(), &mut self.preview_space);
                
                // need to swap the draw context's space for the spawner 'preview space'
                let draw_context = DrawContext {
                    space: &self.preview_space,
                    textures: draw_context.textures,
                    prefabs: draw_context.prefabs,
                    fonts: draw_context.fonts,
                    camera_rect: draw_context.camera_rect,
                    tiles: draw_context.tiles,
                    elapsed_time: draw_context.elapsed_time,
                    default_camera: draw_context.default_camera,
                };

                generic_physics_prop.draw(&draw_context).await;
                // need to immedietly remove the rigid bodies from space because this is a temporary object
                self.preview_space.rigid_body_set.remove(generic_physics_prop.rigid_body_handle, &mut self.preview_space.island_manager, &mut self.preview_space.collider_set,&mut self.preview_space.impulse_joint_set, &mut self.preview_space.multibody_joint_set, true);
                 
            },
            SpawnerCategory::Tile => {
                let tile_save: TileSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let x_index = (rapier_cursor.x / 50.) as usize;
                let y_index = ((rapier_cursor.y +25.) / 50.) as usize;

                let tile: Tile = Tile::from_save(tile_save);

                tile.draw(draw_context.textures, Vector2::new(x_index * 50, y_index * 50));
            }
        }
    }
    
    pub fn spawn(&mut self, area: &mut Area, camera_rect: &Rect, cursor: Vec2, rapier_cursor: Vec2) {
        

        match self.selected_category {
            
            SpawnerCategory::Decoration => {

                let decoration_save: DecorationSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut decoration: Decoration = Decoration::from_save(decoration_save);


                decoration.pos = cursor;

                area.decorations.push(decoration);
                
            },
            SpawnerCategory::Background => {
                let background_save: BackgroundSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut background = Background::from_save(background_save);

                background.pos = cursor;

                area.backgrounds.insert(0, background);
            },

            SpawnerCategory::Prop => {
                let generic_physics_prop_save: PropSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut generic_physics_prop = Prop::from_save(generic_physics_prop_save.clone(), &mut area.space);

                generic_physics_prop.set_pos(vector![rapier_cursor.x + generic_physics_prop_save.size.x / 2., rapier_cursor.y - generic_physics_prop_save.size.y / 2.].into(), &mut area.space);

                area.props.push(Box::new(generic_physics_prop));
            },

            SpawnerCategory::Tile => {
                let tile_save: TileSave = serde_json::from_str(&self.selected_prefab_json).unwrap();
                let tile: Tile = Tile::from_save(tile_save);

                let x_index = (rapier_cursor.x / 50.) as usize;
                let y_index = (rapier_cursor.y / 50.) as usize;
                
                


                area.tiles[x_index][y_index] = Some(tile);
            }
        }
    }


    pub async fn draw_menu(&self, camera_rect: &Rect, textures: &mut TextureLoader, cursor: Vec2) {

        let selected_category = &self.selected_category;

        let mut category_x = 0.;
        for category in SpawnerCategory::iter() {

            let color = match *selected_category == category {
                true => WHITE,
                false => LIGHTGRAY,
            };
            

            draw_text(format!("{}", category).as_str(), category_x, 20., 24., color);

            // this is silly
            category_x += category.to_string().len() as f32 * 12.;
        }
        
        
        


        let prefab_paths = self.menu.prefabs.get(selected_category).unwrap();
        for (index, path) in prefab_paths.iter().enumerate() {

            let color = match index == self.selected_prefab as usize {
                true => GREEN,
                false => WHITE,
            };

            draw_text(&path, 0., ((index) * 20) as f32 + 40., 20., color);
        }

        for (button, _) in &self.prefab_buttons {
            let rect = button.rect;

            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., WHITE);
        }

        for (button, _) in &self.category_buttons {
            let rect = button.rect;

            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., WHITE);
        }

        

    }
}