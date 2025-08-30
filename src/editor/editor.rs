use std::{collections::HashMap, fs::{self, read_to_string}, path::{Path, PathBuf}, time::Instant};

use interceptors_lib::{area::{Area, AreaSave}, background::{Background, BackgroundSave}, clip::Clip, decoration::{Decoration, DecorationSave}, draw_hitbox, font_loader::FontLoader, is_key_released_exclusive, macroquad_to_rapier, mouse_world_pos, prop::{Prop, PropSave}, rapier_mouse_world_pos, rapier_to_macroquad, space::Space, texture_loader::TextureLoader, Prefabs};
use macroquad::{camera::{set_camera, set_default_camera, Camera2D}, color::{GREEN, RED, WHITE}, input::{is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_released, mouse_delta_position, mouse_wheel, KeyCode, MouseButton}, math::{Rect, Vec2}, shapes::{draw_rectangle, draw_rectangle_lines}, text::{draw_text, Font}, window::{next_frame, screen_height, screen_width}};
use nalgebra::{vector, Vector2};
use rapier2d::prelude::{ColliderBuilder, RigidBodyBuilder};
use serde::{de, Deserialize, Serialize};
use strum::{Display, EnumIter};

include!(concat!(env!("OUT_DIR"), "/prefabs.rs"));
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

fn list_dir_entries<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
    let path = path.as_ref(); // keep the original path reference
    let entries = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path()) // convert to full PathBuf
        .filter_map(|p: PathBuf| p.to_str().map(|s| s.to_string())) // PathBuf -> String
        .collect();

    Ok(entries)
}

#[derive(Display, PartialEq, Clone, Copy)]
pub enum Mode {
    PrefabPlacement,
    ClipDefine,
    SetSpawnPoint
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, Clone, Copy)]
pub enum SpawnerCategory {
    Decoration,
    Background,
    Prop
}

pub struct SpawnerMenu {
    pub categories: Vec<SpawnerCategory>,
    pub prefabs: HashMap<SpawnerCategory, Vec<String>>
}

impl SpawnerMenu {
    pub fn new() -> Self {

        let categories = vec![SpawnerCategory::Decoration, SpawnerCategory::Background, SpawnerCategory::Prop];
        let mut prefabs: HashMap<SpawnerCategory, Vec<String>> = HashMap::new();

        prefabs.insert(SpawnerCategory::Decoration, list_dir_entries("prefabs/decorations/").unwrap());
        prefabs.insert(SpawnerCategory::Background, list_dir_entries("prefabs/backgrounds/").unwrap());        
        prefabs.insert(SpawnerCategory::Prop, list_dir_entries("prefabs/generic_physics_props/").unwrap());  

        Self {  
            prefabs,
            categories
        }
    }
}

pub struct Spawner {
    menu: SpawnerMenu,
    selected_prefab: i32, // vector index
    selected_category: i32,
    change: bool,
    selected_prefab_json: String
}

impl Spawner {
    pub async fn new() -> Self {
        let spawner_menu = SpawnerMenu::new();

        let mut spawner = Self {
            menu: spawner_menu,
            selected_prefab: 0,
            selected_category: 0,
            change: true,
            selected_prefab_json: String::new(),
        };

        spawner.load_prefab();

        spawner


    }

    pub fn draw_coords(&self, cursor: Vec2) {

        let rapier_coords = macroquad_to_rapier(&cursor);

        draw_text(&format!("macroquad: {:?}", cursor), 0., screen_height() - 20., 24., WHITE);
        draw_text(&format!("rapier: {:?}", rapier_coords), 0., screen_height() - 40., 24., WHITE);

    }

    pub fn tick(&mut self, area: &mut Area, camera_rect: &Rect, cursor: Vec2, rapier_cursor: Vec2) {


        self.change = false;

        // change selected category
        if is_key_down(KeyCode::Tab) {
            if is_key_released(KeyCode::Right) {
                self.selected_category += 1;

                self.change = true;
            }

            if is_key_released(KeyCode::Left) {
                self.selected_category -= 1;

                self.change = true;
            }
        }


        // change selected prefab
        if is_key_down(KeyCode::Tab) {

            if is_key_released(KeyCode::Up) {
                
                self.selected_prefab -= 1;

                self.change = true;
            }

            if is_key_released(KeyCode::Down) {
                self.selected_prefab += 1;

                self.change = true;
            }
        }

        if self.selected_category > self.menu.categories.len() as i32 - 1 {
            self.selected_category = 0
        }

        if self.selected_category < 0 {
            self.selected_category = self.menu.categories.len() as i32 - 1;
        }

        let category = self.menu.categories.get(self.selected_category as usize).unwrap();

        if self.selected_prefab > self.menu.prefabs.get(category).unwrap().len() as i32 - 1  {
            self.selected_prefab = 0
        }

        if self.selected_prefab < 0 {
            self.selected_prefab = self.menu.prefabs.get(category).unwrap().len() as i32 - 1;
        }

        self.load_prefab();


        if is_key_released_exclusive(&[KeyCode::Space]) {
            self.spawn(area, camera_rect, cursor, rapier_cursor);
        }

        

    }

    pub fn current_category(&self) -> SpawnerCategory {
        self.menu.categories.get(self.selected_category as usize).unwrap().clone()
    }

    pub fn current_prefab(&self) -> String {
        let prefabs = self.menu.prefabs.get(&self.current_category()).unwrap();

        prefabs.get(self.selected_prefab as usize).unwrap().clone()
    }

    pub fn load_prefab(&mut self) {
        if self.change {
            let selected_prefab_path = self.menu.prefabs.get(&self.current_category()).unwrap().get(self.selected_prefab as usize).unwrap();

            self.selected_prefab_json = read_to_string(selected_prefab_path).unwrap();
        }   
    }

    pub async fn draw_preview_spawn(&self, camera_rect: &Rect, textures: &mut TextureLoader, cursor: Vec2, space: &mut Space, rapier_cursor: Vec2, elapsed: web_time::Duration) {

        match self.current_category() {
            
            SpawnerCategory::Decoration => {

                let decoration_save: DecorationSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut decoration: Decoration = Decoration::from_save(decoration_save);

                decoration.pos = cursor;

                decoration.draw(textures, elapsed).await
                
            },
            SpawnerCategory::Background => {
                let background_save: BackgroundSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut background = Background::from_save(background_save);

                background.pos = cursor;

                background.draw(textures, camera_rect).await
            },

            SpawnerCategory::Prop => {
                let generic_physics_prop_save: PropSave = serde_json::from_str(&self.selected_prefab_json).unwrap();

                let mut generic_physics_prop = Prop::from_save(generic_physics_prop_save.clone(), space);

                generic_physics_prop.set_pos(vector![rapier_cursor.x + generic_physics_prop_save.size.x / 2., rapier_cursor.y - generic_physics_prop_save.size.y / 2.].into(), space);

                generic_physics_prop.draw(space, textures).await;

                // need to immedietly remove the rigid bodies from space because this is a temporary object
                space.rigid_body_set.remove(generic_physics_prop.rigid_body_handle, &mut space.island_manager, &mut space.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
                
            }
        }
    }
    
    pub fn spawn(&mut self, area: &mut Area, camera_rect: &Rect, cursor: Vec2, rapier_cursor: Vec2) {
        

        match self.current_category() {
            
            SpawnerCategory::Decoration => {

                dbg!("spawning");
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

                area.props.push(generic_physics_prop);
            }
        }
    }


    pub async fn draw_menu(&self, camera_rect: &Rect, textures: &mut TextureLoader, cursor: Vec2) {

        let category = self.menu.categories.get(self.selected_category as usize).unwrap();
    
        draw_text(&format!("{}", category), 0., 20., 24., WHITE);


        let prefab_paths = self.menu.prefabs.get(category).unwrap();
        for (index, path) in prefab_paths.iter().enumerate() {

            let color = match index == self.selected_prefab as usize {
                true => GREEN,
                false => WHITE,
            };

            draw_text(&path, 0., ((index) * 20) as f32 + 40., 20., color);
        }

        self.draw_coords(cursor);

    }
}

pub struct AreaEditor {
    area: Area,
    textures: TextureLoader,
    spawner: Spawner,
    selected_mode: usize,
    mode_options: Vec<Mode>,
    camera_rect: Rect,
    previous_cursor: Vec2,
    cursor: Vec2,
    clip_point_1: Option<Vec2>,
    clip_point_2: Option<Vec2>,
    last_cursor_move: web_time::Instant,
    prefab_data: Prefabs,
    fonts: FontLoader,
    start: web_time::Instant
}

fn round_to_nearest_50(n: f32) -> f32 {
    (n / 50.0).round() * 50.0
}

impl AreaEditor {

    pub fn highlight_selected_decoration(&mut self) {


        for decoration in &self.area.decorations {

            let rect = Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y);

            if rect.contains(mouse_world_pos(&self.camera_rect)) || rect.contains(self.cursor) {
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3., WHITE);
            }
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
            mode_options: vec![Mode::PrefabPlacement, Mode::ClipDefine, Mode::SetSpawnPoint],
            camera_rect,
            cursor: Vec2::ZERO,
            clip_point_1: None,
            clip_point_2: None,
            previous_cursor: Vec2::ZERO,
            last_cursor_move: web_time::Instant::now(),
            prefab_data: prefabs,
            fonts,
            start: web_time::Instant::now()
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

    pub fn update_cursor_mouse(&mut self) {
        if is_mouse_button_released(MouseButton::Left) {
            self.cursor = mouse_world_pos(&self.camera_rect);

            // snap cursor to top left of decoration if the cursor is there
            // for decoration in &self.area.decorations {
            //     if Rect::new(decoration.pos.x, decoration.pos.y, decoration.size.x, decoration.size.y).contains(mouse_world_pos(&self.camera_rect)) {
            //         self.cursor = decoration.pos;

                    
            //     } 
            // }
        }
    }
    pub fn update_cursor(&mut self) {

        if is_key_down(KeyCode::Tab) {
            return;
        }

        self.previous_cursor = self.cursor.clone();
        
        let cursor_move_amount = match is_key_down(KeyCode::LeftShift) {
            true => 10.,
            false => 1.,
        };

        if self.last_cursor_move.elapsed().as_secs_f32() < 0.1 {
            return;
        }
        

        // let cursor_move_amount = match is_key_down(KeyCode::LeftAlt) {
        //     true =>  {
        //         if self.last_cursor_move.elapsed().as_secs_f32() < 0.1 {
        //             return;
        //         } else {
        //             1.
        //         }
        //     },
        //     false => {
        //         if self.last_cursor_move.elapsed().as_secs_f32() < 0.1 {
        //             return;
        //         } else {
        //             50.
        //         }
        //     },
        // };

        self.last_cursor_move = web_time::Instant::now();


        if is_key_down(KeyCode::Left) {
            

            self.cursor.x -= cursor_move_amount;
        }

        if is_key_down(KeyCode::Right) {

            self.cursor.x += cursor_move_amount;
        }

        if is_key_down(KeyCode::Up) {


            self.cursor.y -= cursor_move_amount;
        }

        if is_key_down(KeyCode::Down) {

            self.cursor.y += cursor_move_amount;
        }

    }

    pub fn current_mode(&self) -> Mode {
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

        if self.current_mode() == Mode::PrefabPlacement {

            let rapier_cursor = self.rapier_cursor();
            
            self.spawner.draw_preview_spawn(&self.camera_rect, &mut self.textures, self.cursor, &mut self.area.space, rapier_cursor, self.start.elapsed()).await;
        }
        
        
        self.draw_cursor();

        self.draw_clip_points();

        self.draw_clips();

        self.highlight_selected_decoration();
        
        set_default_camera();

        

        if self.current_mode() == Mode::PrefabPlacement {
            self.spawner.draw_menu(&self.camera_rect, &mut self.textures, self.cursor).await;
        }



        self.draw_selected_mode();

        next_frame().await
    }

     pub fn draw_cursor(&self) {
        draw_rectangle(self.cursor.x, self.cursor.y, 5., 5., WHITE);
    }


    pub fn clip_tick(&mut self) {
        if self.current_mode() == Mode::ClipDefine {

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
    }

    pub fn tick(&mut self) {

        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::Space) {
                self.create_clip();
            }
        }

        self.update_cursor_mouse();

        self.move_delete();

        self.area.space.step(web_time::Duration::from_secs_f64(0.016));
        

        self.clip_tick();

        self.update_cursor();

        if self.mode_options[self.selected_mode] == Mode::PrefabPlacement {

            let rapier_cursor = self.rapier_cursor();

            self.spawner.tick(&mut self.area, &self.camera_rect, self.cursor, rapier_cursor);
        }
        

        if is_key_down(KeyCode::LeftControl) {
            if is_key_released(KeyCode::S) {

                println!("saving");

                self.save_area();
            }
        }


        self.change_mode();

        self.update_camera();
        
    }

    pub fn set_spawn_point(&mut self) {
        if self.current_mode() != Mode::SetSpawnPoint {
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