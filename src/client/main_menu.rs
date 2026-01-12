use std::path::PathBuf;

use interceptors_lib::{ambiance::Ambiance, button::Button, font_loader::FontLoader, sound_loader::SoundLoader, texture_loader::TextureLoader};
use macroquad::{camera::{Camera2D, set_camera, set_default_camera}, color::{BLACK, WHITE}, input::mouse_position, math::{Rect, vec2}, prelude::{Material, ShaderSource, gl_use_default_material, gl_use_material, load_material}, text::draw_text, texture::{DrawTextureParams, RenderTarget, draw_texture_ex, render_target}, window::{clear_background, next_frame, screen_height, screen_width}};

use crate::{Assets, client::ASSET_PATHS, shaders::{CRT_FRAGMENT_SHADER, CRT_VERTEX_SHADER}};

pub struct MainMenu {
    material: Material,
    render_target: RenderTarget,
    sounds: SoundLoader,
    fonts: FontLoader,
    textures: TextureLoader,
    camera_rect: Rect,
    ui: MainMenuUI,
    ambiance: Vec<Ambiance>
}


impl MainMenu {

    fn start_ambiance(&mut self) {
        for ambiance in &mut self.ambiance {
            ambiance.start_if_stopped(&mut self.sounds);
        }
    }

    pub async fn run(&mut self) -> MainMenuResult {
        loop {

            match self.tick() {
                Some(_) => {},
                None => {},
            }

            self.draw().await;

            next_frame().await


        }
    }
    pub async fn new(
        assets: Assets
    ) -> Self {

        let material = load_material(
            ShaderSource::Glsl {
                vertex: CRT_VERTEX_SHADER,
                fragment: CRT_FRAGMENT_SHADER,
            },
            Default::default(),
        ).unwrap();

        let render_target = render_target(1280, 720);

        let camera_rect = Rect {
            x: 0.,
            y: 0.,
            w: 1280.,
            h: 720.,
        };
        let mut camera = Camera2D::from_display_rect(camera_rect);
        camera.render_target = Some(render_target.clone());
        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

        let ui = MainMenuUI::new();

        let ambiance = vec![
            Ambiance { 
                path: "assets/sounds/radio_chatter.wav".into(), 
                pos: vec2(0., 0.), 
                volume: 1., 
                sound: None 
            }
        ];
        
        Self {
            material: material,
            render_target: render_target,
            sounds: assets.sounds,
            fonts: assets.fonts,
            textures: assets.textures,
            camera_rect: camera_rect,
            ui,
            ambiance
        }
    }

    
    pub fn tick(&mut self) -> Option<MainMenuResult> {
        self.ui.tick();
        self.start_ambiance();

        None
    }
    
    pub async fn draw(&self) {
        let mut camera = Camera2D::from_display_rect(self.camera_rect);
        camera.render_target = Some(self.render_target.clone());
        camera.zoom.y = -camera.zoom.y; 
        set_camera(&camera);
        clear_background(BLACK);
        self.ui.draw();

        draw_text("Interceptors", 50., 50., 35., WHITE);
        
        set_default_camera();
        
        gl_use_material(&self.material);
        draw_texture_ex(
            &self.render_target.texture,
            0., 
            0., 
            WHITE, 
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            }
        );
        gl_use_default_material();

        

        next_frame().await

    }
}

pub enum MainMenuResult {
    Quit,
    Connect
}

pub struct MainMenuUI {
    play_button: Button,    
    quit_button: Button
}

impl MainMenuUI {
    pub fn new() -> Self {
        MainMenuUI {
            play_button: Button::new(
                Rect::new(
                    0., 
                    0., 
                    100., 
                    50.
                ),
                None    
            ),
            quit_button: Button::new(
                Rect::new(
                    0., 
                    0., 
                    100., 
                    50.
                ),
                None
            )   
        }
    }

    pub fn draw(&self) {

    }

    pub fn tick(&mut self) {
        self.play_button.update(mouse_position().into());
        self.quit_button.update(mouse_position().into());
    }
}