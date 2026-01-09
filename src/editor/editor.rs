use std::{fs::{self, read_to_string}, path::PathBuf, process::exit, time::{Duration, Instant}};

use interceptors_lib::{Prefabs, area::{Area, AreaSave}, clip::Clip, decoration::Decoration, draw_hitbox, drawable::{DrawContext, Drawable}, editor_context_menu::EditorContextMenu, font_loader::FontLoader, macroquad_to_rapier, mouse_world_pos, rapier_mouse_world_pos, rapier_to_macroquad, selectable_object_id::{SelectableObject, SelectableObjectId}, texture_loader::TextureLoader};
use log::info;
use macroquad::{camera::{Camera2D, set_camera, set_default_camera}, color::{Color, GRAY, GREEN, RED, WHITE}, input::{KeyCode, MouseButton, is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_released, mouse_delta_position, mouse_wheel}, math::{Rect, Vec2}, shapes::{draw_rectangle, draw_rectangle_lines}, text::draw_text, time::draw_fps, window::{next_frame, screen_height, screen_width}};
use nalgebra::{vector, Vector2};
use rapier2d::{math::Point, prelude::{ColliderBuilder, PointQuery, RigidBodyBuilder, RigidBodyVelocity}};
use strum::Display;

use crate::{editor_input_context::EditorInputContext, editor_mode_select_ui::EditorModeSelectUI, editor_ui_tick_context::EditorUITickContext, layer_toggle_ui::LayerToggleUI, spawner::Spawner};

include!(concat!(env!("OUT_DIR"), "/prefabs.rs"));
include!(concat!(env!("OUT_DIR"), "/assets.rs"));




#[derive(Display, PartialEq, Clone, Copy)]
pub enum EditorMode {
    PrefabPlacement,
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
    layer_toggle_ui: LayerToggleUI,
    input_context: EditorInputContext,
    selection_rect: Option<Rect>,
    selected_released_flag: bool,
    last_mouse_pos: Vec2,
    dragging_object: bool,
    simulate_space: bool,
    undo_checkpoints: Vec<AreaSave>,
    last_checkpoint_save: web_time::Instant,
    last_undo: web_time::Instant,
    modifying: bool,
    last_area_save: AreaSave
}

impl AreaEditor {

    pub fn draw_coords(&self, cursor: Vec2) {

        

        let rapier_coords = macroquad_to_rapier(&cursor);

        draw_text(&format!("macroquad: {:?}", cursor), 0., screen_height() - 20., 24., WHITE);
        draw_text(&format!("rapier: {:?}", rapier_coords), 0., screen_height() - 40., 24., WHITE);

    }

    pub fn get_hovered_object(&mut self, disabled_layers: &Vec<u32>) -> Option<SelectableObjectId> {

        for (clip_index, clip) in self.area.clips.iter().enumerate() { 

            if disabled_layers.contains(&clip.layer) {continue;}

            let clip_collider = self.area.space.collider_set.get(clip.collider_handle).unwrap();

            
            //dbg!(clip_collider.shape().as_cuboid().unwrap().half_extents);
            if clip_collider.shape().as_cuboid().unwrap().contains_point(clip_collider.position(), &Point::new(self.rapier_cursor().x, self.rapier_cursor().y)) {
                
                return Some(SelectableObjectId::Clip(clip_index))
            }
        }
        
        for (decoration_index, decoration) in self.area.decorations.iter().enumerate() {
            if disabled_layers.contains(&decoration.layer) {continue;}

            let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

            if decoration_rect.contains(mouse_world_pos(&self.camera_rect)) || decoration_rect.contains(self.cursor) {
                return Some(SelectableObjectId::Decoration(decoration_index));
            }
        }

        for prop in &self.area.props {
            if disabled_layers.contains(&prop.draw_layer()) {continue;}

            let prop_collider = self.area.space.collider_set.get(prop.as_prop().collider_handle).unwrap();

            if prop_collider.shape().as_cuboid().unwrap().contains_point(prop_collider.position(), &Point::new(self.rapier_cursor().x, self.rapier_cursor().y)) {

                return Some(SelectableObjectId::Prop(prop.id()))
                
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

        let disabled_layers = self.layer_toggle_ui.get_disabled_layers();

        if let Some(hovered_object_id) = self.get_hovered_object(&disabled_layers) {

            let hovered_object = self.area.get_selectable_object_mut(hovered_object_id).unwrap();

            if disabled_layers.contains(&hovered_object.get_layer()) {
                return;
            }

            self.selected_objects.push(hovered_object_id);
        } 
    }

    pub fn update_active_layer_to_selected_object(&mut self) {
        
        // only if one object is selected
        if self.selected_objects.len() != 1 {
            return;
        }

        // dirty hack to only run this when we are selecting a new object
        if !is_mouse_button_released(MouseButton::Left) {
            return;
        }


        let selected_object = if let Some(selected_object_id) = self.selected_objects.get(0) {
            if let Some(selected_object) = self.area.get_selectable_object_mut(*selected_object_id)  {
                selected_object
            } else {
                return;
            }
        } else {
            return;
        };

        // might not be a good idea to be controlling this variable from outside the structure
        self.layer_toggle_ui.active_layer = selected_object.get_layer();



    }

    pub fn drag_object(&mut self, disabled_layers: &Vec<u32>) {

        self.dragging_object = false;  

        if self.current_mode() != EditorMode::Select {return};
        if !is_mouse_button_down(MouseButton::Left) {return};
        if self.get_hovered_object(disabled_layers).is_none() {return};
        if self.selection_rect.is_some() && !self.selected_released_flag {return};

        self.dragging_object = true;
        

        let delta = mouse_world_pos(&self.camera_rect) - self.last_mouse_pos;

        for selected_object_id in &self.selected_objects {
            if let Some(selected_object) = selected_object_id.get_object(&mut self.area.props, &mut self.area.tiles, &mut self.area.decorations, &mut self.area.clips) {
                match selected_object {
                    SelectableObject::Decoration(decoration) => {
    
                        decoration.pos += delta;
                    },
                    SelectableObject::Tile(tile) => {

                    },
                    SelectableObject::Prop(prop) => {
                        let body = self.area.space.rigid_body_set.get_mut(prop.as_prop().rigid_body_handle).unwrap();

                        body.set_vels(RigidBodyVelocity::zero(), false);
                        body.set_angvel(0., false);

                        body.set_position(
                            vector![body.translation().x + delta.x, body.translation().y - delta.y].into(), 
                            true
                        );
                    },
                    SelectableObject::Clip(clip) => {
                        let body = self.area.space.rigid_body_set.get_mut(clip.rigid_body_handle).unwrap();

                        

                        body.set_position(
                            vector![body.translation().x + delta.x, body.translation().y - delta.y].into(), 
                            true
                        );
                    },
                }
            }
        }
        
         
    }

    pub fn highlight_object(&mut self, item: SelectableObjectId, color: Color) {

        let object = match item.get_object(&mut self.area.props, &mut self.area.tiles, &mut self.area.decorations, &mut self.area.clips) {
            Some(object) => object,
            None => return,
        };

        match object {
            SelectableObject::Decoration(decoration) => {


                let decoration_rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

                draw_rectangle_lines(decoration_rect.x, decoration_rect.y, decoration_rect.w, decoration_rect.h, 3., color);
            },

            SelectableObject::Tile(tile) => {

                // the tile itself does not contain its own position so we just do this
                let tile_index = if let SelectableObjectId::Tile(tile_index) = item {
                    tile_index
                } else {
                    panic!("failed to get tile positon")
                };

                let macroquad_pos = rapier_to_macroquad(Vector2::new(tile_index.x as f32 * 50., tile_index.y as f32 * 50.));

                let tile_rect = Rect::new(
                    macroquad_pos.x - 25., 
                    macroquad_pos.y - 25., 
                    50., 
                    50.
                );

                draw_rectangle_lines(tile_rect.x, tile_rect.y, tile_rect.w, tile_rect.h, 3.,  color);

            },

            SelectableObject::Prop(prop) => {

                let prop_pos = self.area.space.rigid_body_set.get(prop.as_prop().rigid_body_handle).unwrap().position();

                let shape = self.area.space.collider_set.get(prop.as_prop().collider_handle).unwrap().shape().as_cuboid().unwrap();

                let macroquad_prop_pos = rapier_to_macroquad(prop_pos.translation.vector);

                let prop_rect = Rect::new(macroquad_prop_pos.x - shape.half_extents.x, macroquad_prop_pos.y - shape.half_extents.y,  shape.half_extents.x * 2., shape.half_extents.y * 2.);

                draw_rectangle_lines(prop_rect.x, prop_rect.y, prop_rect.w, prop_rect.h, 3., color);
            },  
            SelectableObject::Clip(clip) => {
                
                let clip_pos = self.area.space.rigid_body_set.get(clip.rigid_body_handle).unwrap().position();

                let shape = self.area.space.collider_set.get(clip.collider_handle).unwrap().shape().as_cuboid().unwrap();

                let macroquad_clip_pos = rapier_to_macroquad(clip_pos.translation.vector);

                let clip_rect = Rect::new(macroquad_clip_pos.x - shape.half_extents.x, macroquad_clip_pos.y - shape.half_extents.y,  shape.half_extents.x * 2., shape.half_extents.y * 2.);

                draw_rectangle_lines(clip_rect.x, clip_rect.y, clip_rect.w, clip_rect.h, 3., color);


            }
        }
    }

    pub async fn new(area_path: String) -> Self {

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

        let area_json = match read_to_string(&area_path) {
            Ok(area_json) => area_json,
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::NotFound => {

                        info!("Creating new area at path: {}", &area_path);

                        let empty_area_json = serde_json::to_string_pretty(&Area::empty().save()).unwrap();

                        fs::write(&area_path,  &empty_area_json)
                            .map_err(|e| 
                                {
                                    log::error!("Failed to write new area at path: {:?}: {}", area_path, e);
                                    exit(1)
                                }
                            );

                        empty_area_json
                    },
                    _ => {
                        log::error!("Failed to open area path: {:?}", error);
                        exit(1)
                    }
                }
            },
        };

        let area_save: AreaSave = serde_json::from_str(&area_json).unwrap();

        Self {
            area: Area::from_save(area_save.clone(), None, &prefabs),
            textures,
            spawner,
            selected_mode: 0,
            mode_options: vec![EditorMode::Select, EditorMode::PrefabPlacement, EditorMode::SetSpawnPoint, EditorMode::TilePlacement],
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
            dragging_object: false,
            simulate_space: false,
            layer_toggle_ui: LayerToggleUI::new(),
            undo_checkpoints: vec![area_save.clone()],
            last_checkpoint_save: web_time::Instant::now() - Duration::from_secs(50),
            last_undo: web_time::Instant::now(),
            modifying: false,
            last_area_save: area_save
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
        if let Some(hovered_object_id) = self.get_hovered_object(&self.layer_toggle_ui.get_disabled_layers()) {

            let hovered_object = self.area.get_selectable_object_mut(hovered_object_id).unwrap();

            if self.layer_toggle_ui.get_disabled_layers().contains(
                &hovered_object.get_layer()
            ) {
                return;
            }
            self.highlight_object(hovered_object_id, GRAY);
        }
    }

    pub fn highlight_selected_object(&mut self) {

        for selected_object in self.selected_objects.clone() {
            self.highlight_object(selected_object, WHITE);
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

        if !is_key_released(KeyCode::Space) {
            return;
        }

        if self.selection_rect.is_none() {
            return;
        }

        let selection_rect = self.selection_rect.unwrap();

        let clip_point_1 =  Vec2::new(selection_rect.x, selection_rect.y);
        let clip_point_2 = Vec2::new(selection_rect.x + selection_rect.w, selection_rect.y + selection_rect.h);

        let rapier_clip_point_1 = macroquad_to_rapier(&clip_point_1);
        let rapier_clip_point_2 = macroquad_to_rapier(&clip_point_2);

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
                rigid_body_handle: rigid_body,
                context_menu_data: None,
                despawn: false,
                layer: self.layer_toggle_ui.active_layer,
                one_way: false
            }
        );



    }


    pub fn update_camera(&mut self) {
        if mouse_wheel().1 < 0. {
            self.camera_rect.w *= 1.1;
        }

        if mouse_wheel().1 > 0. {

            self.camera_rect.w /= 1.1;
        }

        let ratio = screen_height() / screen_width();

        self.camera_rect.h = self.camera_rect.w * ratio;

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

    

    pub async fn draw(&mut self) {

        let mut camera = Camera2D::from_display_rect(self.camera_rect);
        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

        self.area.draw(
            &mut self.textures, 
            &self.camera_rect, 
            &self.prefab_data, 
            &camera, 
            &self.fonts, 
            self.start.elapsed(), 
            self.layer_toggle_ui.get_invisible_layers()
        ).await;

        

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
        
        self.draw_selection_rect();
        self.highlight_hovered_object();
        self.highlight_selected_object();
        self.step_space();
        

        set_default_camera();
        self.ui.draw(&self.textures);
        self.layer_toggle_ui.draw(&self.fonts, &self.textures);

        self.draw_context_menus();

        if self.current_mode() == EditorMode::PrefabPlacement {
            self.spawner.draw_menu(&self.camera_rect, &mut self.textures, self.cursor).await;
        }

        


        self.draw_selected_mode();

        self.draw_coords(self.cursor);

        draw_fps();

        next_frame().await
    }

    pub fn draw_context_menus(&self) {
        for decoration in &self.area.decorations {
            decoration.draw_editor_context_menu();
        }

        for clip in &self.area.clips {
            clip.draw_editor_context_menu();
        }

        for prop in &self.area.props {
            prop.draw_editor_context_menu();
        }
    }

    pub fn draw_selection_rect(&self) {

        if self.selected_released_flag {
            return
        }

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
                self.selection_rect = Some(Rect::new(mouse_world_pos(&self.camera_rect).x, mouse_world_pos(&self.camera_rect).y, 0., 0.));
                self.selected_released_flag = false;    
            }

            match &mut self.selection_rect {
                Some(selection_rect) => {
                    selection_rect.w = mouse_world_pos(&self.camera_rect).x - selection_rect.x;
                    selection_rect.h = mouse_world_pos(&self.camera_rect).y - selection_rect.y;
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
            let prop_collider = self.area.space.collider_set.get(prop.as_prop().collider_handle).unwrap();

            let prop_macroquad_pos = rapier_to_macroquad(prop_collider.position().translation.vector);

            if selection_rect.contains(prop_macroquad_pos) {
                selected_objects.push(SelectableObjectId::Prop(prop.id()));
            }
        }

        // need to add tiles!

        return selected_objects
    }

    fn mode_select_tick(&mut self) {
        self.drag_object(&self.layer_toggle_ui.get_disabled_layers());
        self.select_object();
        self.rectangle_select();
        self.select_objects_in_rectangle(); 
    }

    

    fn select_objects_in_rectangle(&mut self) {

        let mut new_selected_objects = Vec::new();

        for selected_object_id in self.get_objects_in_selection_rectangle() {

            if self.selected_objects.contains(&selected_object_id) {
                continue;
            }

            let selected_object = self.area.get_selectable_object_mut(selected_object_id).unwrap();

            if self.layer_toggle_ui.get_disabled_layers().contains(&selected_object.get_layer()) {
                continue;
            }
            
            new_selected_objects.push(selected_object_id);
        }


        for object in new_selected_objects {
            self.selected_objects.push(object);
        }

    }

    pub fn editor_mode_tick(&mut self) {
        match self.current_mode() {
            EditorMode::PrefabPlacement => self.mode_prefab_placement_tick(),
            EditorMode::SetSpawnPoint => self.mode_set_spawnpoint_tick(),
            EditorMode::TilePlacement => self.mode_tile_placement_tick(),
            EditorMode::Select => self.mode_select_tick(),
        }
    }

    pub fn update_last_mouse_pos(&mut self) {
        self.last_mouse_pos = mouse_world_pos(&self.camera_rect);
    }

    pub fn step_space(&mut self) {

        self.area.space.step(web_time::Duration::from_secs_f64(0.016));
        
    }

    pub fn update_context_menus(&mut self) {

        for (index, decoration) in self.area.decorations.iter_mut().enumerate() {
            
            let selected = self.selected_objects.contains(&SelectableObjectId::Decoration(index));
            decoration.update_menu(&mut self.area.space, &self.camera_rect, selected);
        }

        for (index, clip) in self.area.clips.iter_mut().enumerate() {
            let selected = self.selected_objects.contains(&SelectableObjectId::Clip(index));            
            clip.update_menu(&mut self.area.space, &self.camera_rect, selected);
        }

        for (index, prop) in self.area.props.iter_mut().enumerate() {
            
            let selected = self.selected_objects.contains(&SelectableObjectId::Prop(prop.id()));
            
            prop.update_menu(&mut self.area.space, &self.camera_rect, selected);

            
        }
    }
    
    pub fn update_modifying_status(&mut self) {

        let area_save = self.area.save();
        if self.last_area_save != area_save {
            self.modifying = true;
        } else {
            self.modifying = false;
        }

        self.last_area_save = area_save

        
    }
    pub fn add_undo_checkpoint(&mut self) {
        let current_area_save = self.area.save();

        // dont create new checkpoints if we undid in the last 1 second so we can spam undo without it constantly creating new checkpoints
        if self.last_undo.elapsed().as_secs() < 1 {
            return;
        }

        if let Some(last_checkpoint) = self.undo_checkpoints.last() {
            
            

            if *last_checkpoint != current_area_save && self.modifying == false {
                self.last_checkpoint_save = web_time::Instant::now();

                self.undo_checkpoints.push(current_area_save);

                //println!("adding checkpoint: {}", self.undo_checkpoints.len());
                
            }
        } else {
            // insert the first checkpoint
            self.undo_checkpoints.push(current_area_save);
        }

        
        if self.undo_checkpoints.len() > 500 {
            let excess = self.undo_checkpoints.len() - 500;
            self.undo_checkpoints.drain(0..excess);
        };

    }

    pub fn undo(&mut self) {

        if is_key_down(KeyCode::LeftControl) && is_key_released(KeyCode::Z) {

            self.last_undo = web_time::Instant::now();
            
            let area_id = self.area.id.clone();

            if let Some(checkpoint) = self.undo_checkpoints.pop() {
                self.area = Area::from_save(
                    checkpoint, 
                    Some(area_id), 
                    &self.prefab_data
                );
            }
            
        }   

        
    }

    pub fn tick(&mut self) {    

        self.update_input_context();
        self.ui.update(&mut EditorUITickContext { selected_mode: &mut self.selected_mode, input_context: self.input_context, simulate_space: &mut self.simulate_space });
        self.layer_toggle_ui.update(self.area.get_drawable_objects_self());
        self.editor_mode_tick();
        
        self.update_cursor();
        self.change_mode();
        self.update_camera();
        self.update_context_menus();
        self.update_active_layer_to_selected_object();
        self.area.despawn_entities();
        self.create_clip();

        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::S) {

                log::info!("Saving level...");

                self.save_area();

                log::info!("Saved");
            }
        }

        self.update_last_mouse_pos();
        self.update_modifying_status();
        self.undo();
        self.add_undo_checkpoint();
        
        
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