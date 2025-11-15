use std::{collections::HashMap, fs::{self, read_to_string}, path::{Path, PathBuf}, str::FromStr, time::Instant};

use interceptors_lib::{Prefabs, area::{Area, AreaSave}, background::{Background, BackgroundSave}, button::Button, clip::Clip, decoration::{Decoration, DecorationSave}, draw_hitbox, drawable::DrawContext, font_loader::FontLoader, is_key_released_exclusive, macroquad_to_rapier, mouse_world_pos, prop::{Prop, PropId, PropSave}, rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, tile::{Tile, TileId, TileSave}};
use ldtk2::When;
use macroquad::{camera::{Camera2D, set_camera, set_default_camera}, color::{Color, GRAY, GREEN, LIGHTGRAY, RED, WHITE}, input::{self, KeyCode, MouseButton, is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_released, mouse_delta_position, mouse_position, mouse_wheel}, math::{Rect, Vec2}, shapes::{draw_rectangle, draw_rectangle_lines}, text::draw_text, texture::{DrawTextureParams, draw_texture_ex}, window::{next_frame, screen_height, screen_width}};
use nalgebra::{vector, Isometry, Isometry2, Vector2};
use rapier2d::{math::Point, parry::shape::Cuboid, prelude::{ColliderBuilder, PointQuery, RigidBodyBuilder}};
use serde::{de, Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::{editor_input_context::EditorInputContext, editor_mode_select_ui::EditorModeSelectUI, editor_ui_tick_context::EditorUITickContext, selectable_object_id::{self, SelectableObject, SelectableObjectId}, spawner::Spawner};

include!(concat!(env!("OUT_DIR"), "/prefabs.rs"));
include!(concat!(env!("OUT_DIR"), "/assets.rs"));




#[derive(Display, PartialEq, Clone, Copy)]
pub enum EditorMode {
    PrefabPlacement,
    ClipDefine,
    SetSpawnPoint,
    TilePlacement, 
    Select
}

pub struct AreaEditor {
    area: Area,
    textures: TextureLoader,
    spawner: Spawner,
    selected_mode: usize,
    mode_options: Vec<EditorMode>,
    camera_rect: Rect,
    previous_cursor: Vec2,
    cursor: Vec2,
    clip_point_1: Option<Vec2>,
    clip_point_2: Option<Vec2>,
    last_cursor_move: web_time::Instant,
    prefab_data: Prefabs,
    fonts: FontLoader,
    start: web_time::Instant,
    selected_objects: Vec<SelectableObjectId>,
    ui: EditorModeSelectUI,
    input_context: EditorInputContext,
    selection_rect: Option<Rect>,
    selected_released_flag: bool,
    last_mouse_pos: Vec2,
    dragging_object: bool
}

impl AreaEditor {

    pub fn get_hovered_object(&mut self) -> Option<SelectableObjectId> {

        for (decoration_index, decoration) in self.area.decorations.iter().enumerate() {
            let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

            if decoration_rect.contains(mouse_world_pos(&self.camera_rect)) || decoration_rect.contains(self.cursor) {
                return Some(SelectableObjectId::Decoration(decoration_index));
            }
        }

        for prop in &self.area.props {
            let prop_collider = self.area.space.collider_set.get(prop.collider_handle).unwrap();

            if prop_collider.shape().as_cuboid().unwrap().contains_point(prop_collider.position(), &Point::new(self.rapier_cursor().x, self.rapier_cursor().y)) {

                return Some(SelectableObjectId::Prop(prop.id))
                
            }
        }

        let rapier_mouse_pos = self.rapier_cursor();

        if let Some(tile) = self.area.get_tile_at_position_mut(Vector2::new(rapier_mouse_pos.x - 25., rapier_mouse_pos.y - 25.)) {
            return Some(SelectableObjectId::Tile(Vector2::new((rapier_mouse_pos.x / 50.) as usize, (rapier_mouse_pos.y / 50.) as usize)))
        } 


        return None;
    }

    pub fn select_object(&mut self) {
        if !is_mouse_button_released(MouseButton::Left) {
            return;
        }

        if !is_key_down(KeyCode::LeftControl) {
            self.selected_objects.clear();
        }

        if let Some(hovered_object) = self.get_hovered_object() {
            self.selected_objects.push(hovered_object);
        }

        

        
    }

    pub fn drag_object(&mut self) {

        self.dragging_object = false;  

        if self.current_mode() != EditorMode::Select {return};
        if !is_mouse_button_down(MouseButton::Left) {return};
        if self.get_hovered_object().is_none() {return};
        if self.selection_rect.is_some() {return};

        self.dragging_object = true;
        

        let delta = mouse_world_pos(&self.camera_rect) - self.last_mouse_pos;

        for selected_object_id in &self.selected_objects {
            if let Some(selected_object) = selected_object_id.get_object(&mut self.area.props, &mut self.area.tiles, &mut self.area.decorations) {
                match selected_object {
                    SelectableObject::Decoration(decoration) => {
                        
                        decoration.pos += delta;
                    },
                    SelectableObject::Tile(tile) => {

                    },
                    SelectableObject::Prop(prop) => {
                        let body = self.area.space.rigid_body_set.get_mut(prop.rigid_body_handle).unwrap();

                        body.set_position(
                            vector![body.translation().x + delta.x, body.translation().y + delta.y].into(), 
                            true
                        );
                    },
                }
            }
        }
        
         
    }

    pub fn highlight_object(&mut self, item: SelectableObjectId) {


        match item {
            SelectableObjectId::Decoration(decoration_index) => {
                let decoration = self.area.decorations.get(decoration_index).unwrap();

                let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

                draw_rectangle_lines(decoration_rect.x, decoration_rect.y, decoration_rect.w, decoration_rect.h, 3., WHITE);
            },

            SelectableObjectId::Tile(tile_index) => {
                let tile = self.area.get_tile_index(tile_index);



                if let Some(tile) = tile {

                    let macroquad_pos = rapier_to_macroquad(Vector2::new(tile_index.x as f32 * 50., tile_index.y as f32 * 50.));

                    let tile_rect = Rect::new(
                        macroquad_pos.x - 25., 
                        macroquad_pos.y - 25., 
                        50., 
                        50.
                    );

                    draw_rectangle_lines(tile_rect.x, tile_rect.y, tile_rect.w, tile_rect.h, 3., WHITE);
                }
            },

            SelectableObjectId::Prop(prop_id) => {
                let prop = self.area.props.iter().find(|prop| {prop_id == prop.id}).unwrap();

                let prop_pos = self.area.space.rigid_body_set.get(prop.rigid_body_handle).unwrap().position();

                let shape = self.area.space.collider_set.get(prop.collider_handle).unwrap().shape().as_cuboid().unwrap();

                let macroquad_prop_pos = rapier_to_macroquad(prop_pos.translation.vector);

                let prop_rect = Rect::new(macroquad_prop_pos.x - shape.half_extents.x, macroquad_prop_pos.y - shape.half_extents.y,  shape.half_extents.x * 2., shape.half_extents.y * 2.);

                draw_rectangle_lines(prop_rect.x, prop_rect.y, prop_rect.w, prop_rect.h, 3., WHITE);
            },  
        }
    }

    pub async fn new() -> Self {

        let mut prefabs = Prefabs::new();

        for prefab_path in PREFAB_PATHS {
            prefabs.load_prefab_data(prefab_path).await
        }

        let mut textures = TextureLoader::new();

        let mut fonts = FontLoader::new();

        for asset in ASSET_PATHS {

            if asset.ends_with(".png") {
                textures.load(PathBuf::from(asset)).await;
            }

            if asset.ends_with(".ttf") {
                fonts.load(PathBuf::from(asset)).await
            }
        }

        let spawner = Spawner::new().await;

        let camera_rect = Rect {
            x: 0.,
            y: 0.,
            w: 1280.,
            h: 720.,
        };

        let area_json = read_to_string("areas/forest.json").unwrap();
        let area_save: AreaSave = serde_json::from_str(&area_json).unwrap();

        Self {
            area: Area::from_save(area_save, None, &prefabs),
            textures,
            spawner,
            selected_mode: 0,
            mode_options: vec![EditorMode::Select, EditorMode::PrefabPlacement, EditorMode::ClipDefine, EditorMode::SetSpawnPoint, EditorMode::TilePlacement],
            camera_rect,
            cursor: Vec2::ZERO,
            clip_point_1: None,
            clip_point_2: None,
            previous_cursor: Vec2::ZERO,
            last_cursor_move: web_time::Instant::now(),
            prefab_data: prefabs,
            fonts,
            start: web_time::Instant::now(),
            selected_objects: Vec::new(),
            ui: EditorModeSelectUI::new(),
            input_context: EditorInputContext::World,
            selection_rect: None,
            selected_released_flag: false,
            last_mouse_pos: Vec2::ZERO,
            dragging_object: false
        }
    }

    pub fn update_input_context(&mut self) {
        if self.ui.hovered() {
            self.input_context = EditorInputContext::EditorModeMenu
        } else if self.spawner.hovered() {
            self.input_context = EditorInputContext::SpawnerMenu
        } 
        else {
            self.input_context = EditorInputContext::World;
        }
    }

    pub fn rapier_cursor(&self) -> Vec2 {
        macroquad_to_rapier(&self.cursor)
    }

    pub fn move_delete(&mut self) {
        
        let mut decorations_remove: Vec<Decoration> = Vec::new();

        for decoration in &mut self.area.decorations {

            if Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y).contains(self.previous_cursor) || self.cursor == decoration.pos {

                if is_key_down(KeyCode::Q) && self.cursor != self.previous_cursor {
                    decoration.pos = self.cursor;

                    break;
                }

                if is_key_released(KeyCode::Delete) {
                    decorations_remove.push(decoration.clone());

                    break;
                }
                 
            }
        }

        self.area.decorations.retain(|x| {!decorations_remove.contains(x)});
    }

    pub fn delete(&mut self) {
        
    }

    pub fn highlight_hovered_object(&mut self) {
        if let Some(hovered_object) = self.get_hovered_object() {
            self.highlight_object(hovered_object);
        }
    }

    pub fn highlight_selected_object(&mut self) {

        for selected_object in self.selected_objects.clone() {
            self.highlight_object(selected_object);
        }
    }

    pub fn snap_cursor(&mut self) {
        if !is_key_down(KeyCode::LeftShift) {
            return;
        }

        let bounding_box = self.spawner.get_snapping_bounding_box(self.cursor); 

        if let Some(bounding_box)= bounding_box {
            for decoration in &self.area.decorations {
                let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

                // Left snap
                if bounding_box.x < decoration_rect.center().x 
                && (decoration_rect.left() - bounding_box.right()) < bounding_box.size().x / 2.
                && (bounding_box.center().y - decoration_rect.center().y).abs() < bounding_box.size().y / 2.{

                    self.cursor.x = decoration_rect.left() - bounding_box.size().x;
                    self.cursor.y = decoration_rect.y;

                }

                
                // Right snap
                else if bounding_box.x > decoration_rect.center().x 
                && (bounding_box.left() - decoration_rect.right()) < bounding_box.size().x / 2.
                && (bounding_box.center().y - decoration_rect.center().y).abs() < bounding_box.size().y / 2. {

                    self.cursor.x = decoration_rect.right();
                    self.cursor.y = decoration_rect.y;
                }

                // Top snap
                else if bounding_box.y < decoration_rect.top() 
                && (decoration_rect.top() - bounding_box.bottom()) < bounding_box.size().y 
                && (decoration_rect.center().x - bounding_box.center().x).abs() < bounding_box.size().x / 2. {

                    self.cursor.x = decoration_rect.x;
                    self.cursor.y = decoration_rect.y - bounding_box.size().y;
                }

                // Bottom snap
                else if bounding_box.top() > decoration_rect.bottom() 
                && (bounding_box.top() - decoration_rect.bottom()) < bounding_box.size().y 
                && (decoration_rect.center().x - bounding_box.center().x).abs() < bounding_box.size().x / 2. {

                    self.cursor.x = decoration_rect.x;
                    self.cursor.y = decoration_rect.bottom()
                }
            }

        };
    }

    pub fn update_cursor(&mut self) {

        self.cursor = mouse_world_pos(&self.camera_rect);

        self.snap_cursor();
        
    }

    pub fn current_mode(&self) -> EditorMode {
        self.mode_options.get(self.selected_mode).unwrap().clone()
    }

    pub fn save_area(&self) {

        std::fs::write(
            "areas/forest.json", 
            serde_json::to_string_pretty(
                &self.area.save()
            ).unwrap()
        ).unwrap();
    }

    pub fn draw_selected_mode(&self) {


        draw_text(&format!("{}", self.current_mode()), screen_width() - 200., 20., 24., WHITE);
    }

    pub fn create_clip(&mut self) {

        if !(self.clip_point_1.is_some() && self.clip_point_2.is_some()) {
            return;
        }

        let rapier_clip_point_1 =  &self.clip_point_1.unwrap();
        let rapier_clip_point_2 = &self.clip_point_2.unwrap();

        let x_hx = (rapier_clip_point_2.x - rapier_clip_point_1.x) / 2.;
        let y_hx = (rapier_clip_point_1.y - rapier_clip_point_2.y) / 2.;

        let rigid_body = self.area.space.rigid_body_set.insert(
            RigidBodyBuilder::fixed().position(vector![rapier_clip_point_1.x + x_hx, rapier_clip_point_1.y - y_hx].into())
        );
        let collider = self.area.space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(x_hx, y_hx),
            rigid_body,
            &mut self.area.space.rigid_body_set
        );

        self.area.clips.push(
            Clip {
                collider_handle: collider,
                rigid_body_handle: rigid_body
            }
        );



    }


    pub fn update_camera(&mut self) {
        if mouse_wheel().1 < 0. {
            self.camera_rect.w *= 1.1;
            self.camera_rect.h *= 1.1;
        }

        if mouse_wheel().1 > 0. {

            self.camera_rect.w /= 1.1;
            self.camera_rect.h /= 1.1;
        }

        if !is_key_down(KeyCode::LeftAlt) {
            return;
        }

        let camera_speed = match is_key_down(KeyCode::LeftShift) {
            true => 30.,
            false => 5.,
        };


        if is_key_down(KeyCode::W) {
            self.camera_rect.y -= camera_speed;
        }

        if is_key_down(KeyCode::S) {
            self.camera_rect.y += camera_speed;
        }
        
        if is_key_down(KeyCode::A) {
            self.camera_rect.x -= camera_speed;
        }

        if is_key_down(KeyCode::D) {
            self.camera_rect.x += camera_speed;
        }

        if is_mouse_button_down(MouseButton::Middle) {
            self.camera_rect.x += mouse_delta_position().x * 200.;
            self.camera_rect.y += mouse_delta_position().y * 200.;
        }
    }
    
    pub fn draw_clip_points(&self) {
        if let Some(clip_point_1) = self.clip_point_1 {

            let macroquad_pos = rapier_to_macroquad(Vector2::new(clip_point_1.x, clip_point_1.y));

            draw_rectangle(macroquad_pos.x, macroquad_pos.y, 10., 10., RED);
        }

        if let Some(clip_point_2) = self.clip_point_2 {

            let macroquad_pos = rapier_to_macroquad(Vector2::new(clip_point_2.x, clip_point_2.y));

            draw_rectangle(macroquad_pos.x, macroquad_pos.y, 10., 10., GREEN);
        }
    }

    pub fn draw_clips(&self) {

        let mut color = WHITE;

        color.a = 0.2;

        for clip in &self.area.clips {

            draw_hitbox(&self.area.space, clip.rigid_body_handle, clip.collider_handle, color);
        }
    }

    pub async fn draw(&mut self) {

        let mut camera = Camera2D::from_display_rect(self.camera_rect);
        camera.zoom.y = -camera.zoom.y;



        set_camera(&camera);

        self.area.draw(&mut self.textures, &self.camera_rect, &self.prefab_data, &camera, &self.fonts, self.start.elapsed()).await;

        let draw_context = DrawContext {
            space: &self.area.space,
            textures: &self.textures,
            prefabs: &self.prefab_data,
            fonts: &self.fonts,
            camera_rect: &self.camera_rect,
            tiles: &self.area.tiles,
            elapsed_time: &self.start.elapsed(),
            default_camera: &camera,
        };

        if self.current_mode() == EditorMode::PrefabPlacement {

            let rapier_cursor = self.rapier_cursor();
            
            self.spawner.draw_preview_spawn(&draw_context, self.cursor, rapier_cursor).await;
        }

        
        
        
        self.draw_cursor();

        self.draw_clip_points();

        self.draw_clips();

        self.highlight_hovered_object();

        self.highlight_selected_object();
        
        set_default_camera();

        self.ui.draw(&self.textures);

        

        if self.current_mode() == EditorMode::PrefabPlacement {
            self.spawner.draw_menu(&self.camera_rect, &mut self.textures, self.cursor).await;
        }

        self.draw_selection_rect();


        self.draw_selected_mode();

        next_frame().await
    }

    pub fn draw_selection_rect(&self) {

        match self.selection_rect {
            Some(selection_rect) => {
                draw_rectangle_lines(
                    selection_rect.x, 
                    selection_rect.y, 
                    selection_rect.w, 
                    selection_rect.h, 
                    3., 
                    WHITE
                );
            },
            None => {},
        }
        
    }

    pub fn draw_cursor(&self) {
        draw_rectangle(self.cursor.x, self.cursor.y, 5., 5., WHITE);
    }
    
    pub fn draw_mode_selection_buttons(&self) {

    }

    fn mode_prefab_placement_tick(&mut self) {
        let rapier_cursor = self.rapier_cursor();

        self.spawner.tick(&mut self.area, &self.camera_rect, self.cursor, rapier_cursor, self.input_context);
    }

    fn mode_clip_define_tick(&mut self) {
        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::Space) {
                self.create_clip();
            }
        }

        if !is_key_released(KeyCode::Space) {
            return;
        }

        if self.clip_point_1.is_none() {
            self.clip_point_1 = Some(self.rapier_cursor())
        } else if self.clip_point_1.is_some() && self.clip_point_2.is_none() {
            self.clip_point_2 = Some(self.rapier_cursor())
        } else {
            self.clip_point_1 = Some(self.rapier_cursor());

            self.clip_point_2 = None
        }
    }

    fn mode_set_spawnpoint_tick(&mut self) {

    
    }

    fn mode_tile_placement_tick(&mut self) {

    }

    pub fn rectangle_select(&mut self) {

        if is_mouse_button_released(MouseButton::Left) {

            self.selected_released_flag = true;
        }

        if self.selected_released_flag && self.selection_rect.is_some() {
            let too_small = self.selection_rect.unwrap().size().abs().length() < 20.;

            if too_small {
                self.selection_rect = None;
            }
        }

        if self.dragging_object {return;}
        
        if is_mouse_button_down(MouseButton::Left) {

            // new selection rect
            if self.selected_released_flag {
                self.selection_rect = Some(Rect::new(mouse_position().0, mouse_position().1, 0., 0.));
                self.selected_released_flag = false;    
            }

            match &mut self.selection_rect {
                Some(selection_rect) => {
                    selection_rect.w = mouse_position().0 - selection_rect.x;
                    selection_rect.h = mouse_position().1 - selection_rect.y;
                },
                None => {
                    // NOT POSSIBLE (meme)
                },
            }

        }
        


    }

    pub fn get_objects_in_selection_rectangle(&mut self) -> Vec<SelectableObjectId> {

        let mut selected_objects = Vec::new();

        let selection_rect = match &self.selection_rect {
            Some(selection_rect) => selection_rect,
            None => return selected_objects,
        };

        for (decoration_index, decoration) in self.area.decorations.iter().enumerate() {
            let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

            if selection_rect.overlaps(&decoration_rect) {
                selected_objects.push(SelectableObjectId::Decoration(decoration_index));
            }
        }

        for prop in &self.area.props {
            let prop_collider = self.area.space.collider_set.get(prop.collider_handle).unwrap();

            let prop_macroquad_pos = rapier_to_macroquad(prop_collider.position().translation.vector);

            if selection_rect.contains(prop_macroquad_pos) {
                selected_objects.push(SelectableObjectId::Prop(prop.id));
            }
        }

        // need to add tiles!

        return selected_objects
    }

    fn mode_select_tick(&mut self) {
        self.drag_object();
        self.select_object();
        self.rectangle_select();
        self.select_objects_in_rectangle();
    }

    fn select_objects_in_rectangle(&mut self) {

        let mut new_selected_objects = Vec::new();
        for selected_object in self.get_objects_in_selection_rectangle() {
            if !self.selected_objects.contains(&selected_object) {
                new_selected_objects.push(selected_object);
            }
        }


        for object in new_selected_objects {
            self.selected_objects.push(object);
        }

    }

    pub fn editor_mode_tick(&mut self) {
        match self.current_mode() {
            EditorMode::PrefabPlacement => self.mode_prefab_placement_tick(),
            EditorMode::ClipDefine => self.mode_clip_define_tick(),
            EditorMode::SetSpawnPoint => self.mode_set_spawnpoint_tick(),
            EditorMode::TilePlacement => self.mode_tile_placement_tick(),
            EditorMode::Select => self.mode_select_tick(),
        }
    }

    pub fn update_last_mouse_pos(&mut self) {
        self.last_mouse_pos = mouse_world_pos(&self.camera_rect);
    }
    pub fn tick(&mut self) {    

        self.update_input_context();
        self.ui.update(&mut EditorUITickContext { selected_mode: &mut self.selected_mode, input_context: self.input_context });
        self.editor_mode_tick();
        self.area.space.step(web_time::Duration::from_secs_f64(0.016));
        self.update_cursor();
        self.change_mode();
        self.update_camera();
        

        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::S) {

                println!("saving");

                self.save_area();
            }
        }

        self.update_last_mouse_pos();
        
        
    }

    pub fn set_spawn_point(&mut self) {
        if self.current_mode() != EditorMode::SetSpawnPoint {
            return;
        }

        if is_mouse_button_released(macroquad::input::MouseButton::Left) {
            self.area.spawn_point = rapier_mouse_world_pos(&self.camera_rect);
        }
    }

    pub fn change_mode(&mut self) {
        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::Right) {
                self.selected_mode += 1;

                if self.selected_mode > self.mode_options.len() - 1 {
                    self.selected_mode = 0
                }
            }

            if is_key_released(KeyCode::Left) {
                if self.selected_mode == 0 {
                    self.selected_mode = self.mode_options.len() - 1;

                } else {
                    self.selected_mode -= 1;
                }
            }
        }
        
    }

    pub async fn run(&mut self) {

        loop {
            self.tick();
            self.draw().await
        }
        
    } 
}