use std::path::PathBuf;

use macroquad::{color::WHITE, math::{Rect, Vec2}, texture::{draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::{drawable::Drawable, editor_context_menu::{DataEditorContext, EditorContextMenu, EditorContextMenuData}, space::Space};

// literally just a sprite with position and size
#[derive(Clone, PartialEq)]
pub struct Decoration {
    pub pos: Vec2, // macroquad pos
    pub sprite_path: Option<PathBuf>,
    pub size: Vec2,
    pub frame_duration: Option<web_time::Duration>,
    pub animated_sprite_paths: Option<Vec<PathBuf>>,
    pub layer: u32,
    pub editor_context_menu: Option<EditorContextMenuData>,
    pub despawn: bool
}

impl Decoration {

    pub fn editor_draw(&self) {
        
        self.draw_editor_context_menu();
    }

    pub fn mark_despawn(&mut self) {
        self.despawn = true;
    }

    pub fn despawn_callback(&mut self) {
        
    }
    pub fn from_save(save: DecorationSave) -> Self {

        let frame_duration = match save.frame_duration {
            Some(dur) => Some(web_time::Duration::from_secs_f32(dur)),
            None => None,
        };
            
        Self {
            pos: save.pos,
            sprite_path: save.sprite_path,
            animated_sprite_paths: save.animated_sprite_paths,
            size: save.size,
            frame_duration,
            layer: save.layer,
            editor_context_menu: None,
            despawn: false
        }
    }

    pub fn save(&self) -> DecorationSave {

        let frame_duration = match self.frame_duration {
            Some(duration) => Some(duration.as_secs_f32()),
            None => None,
        };
        
        DecorationSave {
            pos: self.pos,
            size: self.size,
            sprite_path: self.sprite_path.clone(),
            animated_sprite_paths: self.animated_sprite_paths.clone(),
            frame_duration: frame_duration,
            layer: self.layer
        }
    }
    
}

impl EditorContextMenu for Decoration {
    fn context_menu_data_mut(&mut self) -> &mut Option<crate::editor_context_menu::EditorContextMenuData> {
        &mut self.editor_context_menu
    }

    fn data_editor_export(&self, _ctx: &DataEditorContext) -> Option<String> {
        let json_string = serde_json::to_string_pretty(&self.save()).unwrap();
        
        Some(json_string)
    }

    fn data_editor_import(&mut self, json: String, _ctx: &mut DataEditorContext) {
        
        
        match serde_json::from_str(&json) {
            Ok(decoration_save) => {
                *self = Decoration::from_save(decoration_save);
            },
            Err(_) => {return;},
        }

    }

    fn layer(&mut self) -> Option<&mut u32> {
        Some(&mut self.layer)
    }

    fn object_bounding_box(&self, space: Option<&Space>) -> macroquad::prelude::Rect {
        Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y)
    }

    fn context_menu_data(&self) -> &Option<EditorContextMenuData> {
        &self.editor_context_menu
    }
    
    fn despawn(&mut self) -> Option<&mut bool> {
        Some(&mut self.despawn)
    }
}

#[async_trait::async_trait]
impl Drawable for Decoration {
    async fn draw(&mut self, draw_context: &crate::drawable::DrawContext) {
        let sprite_path = match &self.frame_duration {
            Some(frame_duration) => {
                let current_frame = (
                    (
                        draw_context.elapsed_time.as_secs_f32() % (frame_duration.as_secs_f32() * self.animated_sprite_paths.as_ref().unwrap().len() as f32)
                    ) / frame_duration.as_secs_f32()
                ) as usize;

                &self.animated_sprite_paths.as_ref().unwrap()[current_frame]
            },
            None => {
                self.sprite_path.as_ref().unwrap()
            },
        };

        let texture = draw_context.textures.get(sprite_path);

        draw_texture_ex(
            texture, 
            self.pos.x, 
            self.pos.y, 
            WHITE, 
            DrawTextureParams {
                dest_size: Some(self.size),
                source: None,
                rotation: 0.,
                flip_x: false,
                flip_y: false,
                pivot: None,
            }
        );
    }

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DecorationSave {
    pub pos: Vec2,
    pub size: Vec2,
    #[serde(default)]
    pub sprite_path: Option<PathBuf>,
    #[serde(default)]
    pub animated_sprite_paths: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub frame_duration: Option<f32>,
    #[serde(default)]
    pub layer: u32
}