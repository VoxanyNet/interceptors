
use interceptors_lib::{Assets, ambiance::Ambiance, button::Button, font_loader::FontLoader, load_assets, macroquad_to_rapier, sound_loader::SoundLoader, texture_loader::ClientTextureLoader};
use macroquad::{audio::{PlaySoundParams, Sound, play_sound, stop_sound}, camera::{Camera2D, set_camera, set_default_camera}, color::{BLACK, WHITE}, input::mouse_position, math::{Rect, Vec2, vec2}, prelude::{Material, ShaderSource, gl_use_default_material, gl_use_material, load_material}, rand::RandomRange, text::{TextParams, draw_text, draw_text_ex}, texture::{DrawTextureParams, RenderTarget, draw_texture_ex, render_target}, window::{clear_background, next_frame, screen_height, screen_width}};

use crate::{shaders::{CRT_FRAGMENT_SHADER, CRT_VERTEX_SHADER}};

pub struct MainMenu {
    material: Material,
    render_target: RenderTarget,
    sound_loader: SoundLoader,
    fonts: FontLoader,
    textures: ClientTextureLoader,
    camera_rect: Rect,
    ui: MainMenuUI,
    ambiance: Vec<Ambiance>,
    played_radio_chatter: bool,
    start: web_time::Instant,
    sounds: Vec<Sound>
}


impl MainMenu {


    pub fn draw_coords(&self, cursor: Vec2) {

        let rapier_coords = macroquad_to_rapier(&cursor);
        
        draw_text(&format!("{:?}", cursor), 0., screen_height() - 20., 24., WHITE);
        draw_text(&format!("{:?}", rapier_coords), 0., screen_height() - 40., 24., WHITE);
    }

    async fn reload_textures(&mut self) {

        self.textures = load_assets().await.textures
    }

    fn draw_cursor(textures: &ClientTextureLoader) {
        let cursor_pos: Vec2 = mouse_position().into();

        let texture = textures.get(&"assets/cursor.png".into());
        draw_texture_ex(
            texture, 
            cursor_pos.x - 10., 
            cursor_pos.y, 
            WHITE,
            DrawTextureParams {
                dest_size: vec2(texture.size().x * 2., texture.size().y * 2.).into(),
                source: None,
                rotation: 0.,
                flip_x: false,
                flip_y: false,
                pivot: None,
            }
        );
    }
    fn update_window_size(&mut self) {
        let ratio = screen_width() / screen_height();
        self.camera_rect.h = 720.;
        self.camera_rect.w = self.camera_rect.h * ratio;
    }

    fn start_ambiance(&mut self) {
        for ambiance in &mut self.ambiance {
            ambiance.start_if_stopped(&mut self.sound_loader);
        }
    }

    pub async fn run(&mut self) -> MainMenuResult {
        loop {

            match self.tick() {
                Some(result) => {
                    return result
                },
                None => {},
            }


            self.draw().await;

            next_frame().await


        }
    }

    fn play_startup_sounds(&mut self) {

        let sound = self.sound_loader.get("assets/sounds/crt_on_button.wav".into());
        self.sounds.push(sound.clone());

        play_sound(
            sound, 
            PlaySoundParams {
                looped: false,
                volume: 1.,
            }
        );

        let sound = self.sound_loader.get("assets/sounds/hard_drive_spinning.wav".into());
        self.sounds.push(sound.clone());
        
        play_sound(
            sound,
            PlaySoundParams {
                looped: true,
                volume: 0.3,
            }
        );

        let sound = self.sound_loader.get("assets/sounds/drive_spin_up.wav".into());
        self.sounds.push(sound.clone());

        play_sound(
            sound, 
            PlaySoundParams {
                looped: false,
                volume: 0.3,
            }
        );
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

        ];
        
        let mut menu = Self {
            material: material,
            render_target: render_target,
            sound_loader: assets.sounds,
            fonts: assets.fonts,
            textures: assets.textures,
            camera_rect: camera_rect,
            ui,
            ambiance,
            played_radio_chatter: false,
            start: web_time::Instant::now(),
            sounds: Vec::new()

            
        };

        menu.play_startup_sounds();

        menu
    }

    fn play_radio_chatter(&mut self) {
        if !self.played_radio_chatter && RandomRange::gen_range(0, 2) == 1 {

            self.played_radio_chatter = true;

            play_sound(
                self.sound_loader.get("assets/sounds/radio_chatter.wav".into()),
                PlaySoundParams {
                    looped: false,
                    volume: 0.1,
                }
            );

        }

    }
    pub fn tick(&mut self) -> Option<MainMenuResult> {
        self.ui.tick(&self.sound_loader);

        if self.ui.play_button.down {
            self.stop_sounds();
            return Some(MainMenuResult::Connect) 
        }

        if self.ui.quit_button.released {
            self.stop_sounds();
            return Some(MainMenuResult::Quit);
        }
        self.start_ambiance();
        //self.play_radio_chatter();
        self.update_window_size();

        

        None
    }

    fn draw_title(&self) {
        draw_text_ex("INTERCEPTORS", 70., 100., TextParams {
            font: Some(&self.fonts.get("assets/fonts/FuturaHeavy.ttf".into())),
            font_size: 50,        
            rotation: 0.,
            color: WHITE,
            ..Default::default()
        });

    }

    pub fn stop_sounds(&mut self) {
        for sound in &self.sounds {
            stop_sound(sound);
        }
    }
    
    pub async fn draw(&self) {
        let mut camera = Camera2D::from_display_rect(self.camera_rect);
        camera.render_target = Some(self.render_target.clone());
        camera.zoom.y = -camera.zoom.y; 
        set_camera(&camera);
        clear_background(BLACK);
        self.ui.draw(&self.fonts);


        self.draw_title();
        //self.draw_coords(mouse_position().into());

        Self::draw_cursor(&self.textures);
        
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
    connect_text: String,
    quit_text: String,
    play_button: Button,    
    quit_button: Button,
    should_beep_on_button_hover: bool
}

impl MainMenuUI {
    pub fn new() -> Self {

        MainMenuUI {
            play_button: Button::new(
                Rect::new(
                    65., 
                    370., 
                    100., 
                    40.
                ),
                None    
            ),
            quit_button: Button::new(
                Rect::new(
                    65., 
                    420., 
                    100., 
                    40.
                ),
                None
            ),
            connect_text: "connect".into(),
            quit_text: "quit".into(),   
            should_beep_on_button_hover: true
        }
    }

    pub fn draw(&self, fonts: &FontLoader) {
        // draw_text(&self.connect_text, 70., 400., 50., WHITE);
        // draw_text(&self.quit_text, 70., 450., 50., RED);

        if self.play_button.hovered {
            draw_text_ex(
                ">", 
                self.play_button.rect.x - 20., 
                self.play_button.rect.y + 30.,
                TextParams {
                    font: Some(&fonts.get("assets/fonts/FuturaHeavy.ttf".into())),
                    font_size: 30, 
                    color: WHITE,
                    ..Default::default()
                }
            );
        }

        if self.quit_button.hovered {
            draw_text_ex(
                ">", 
                self.quit_button.rect.x - 20., 
                self.quit_button.rect.y + 30.,
                TextParams {
                    font: Some(&fonts.get("assets/fonts/FuturaHeavy.ttf".into())),
                    font_size: 30, 
                    color: WHITE,
                    ..Default::default()
                }
            );
        }


        draw_text_ex(&self.connect_text, 70., 400., TextParams {
            font: Some(&fonts.get("assets/fonts/FuturaHeavy.ttf".into())),
            font_size: 30,        
            rotation: 0.,
            color: WHITE,
            ..Default::default()
        });

        draw_text_ex(&self.quit_text, 70., 450., TextParams {
            font: Some(&fonts.get("assets/fonts/FuturaHeavy.ttf".into())),
            font_size: 30,        
            rotation: 0.,
            color: WHITE,
            ..Default::default()
        });

        // draw_rectangle_lines(self.play_button.rect.x, self.play_button.rect.y, self.play_button.rect.w, self.play_button.rect.h, 5., WHITE);
        // draw_rectangle_lines(self.quit_button.rect.x, self.quit_button.rect.y, self.quit_button.rect.w, self.quit_button.rect.h, 5., WHITE);

    }

    pub fn tick(&mut self, sounds: &SoundLoader) {
        self.play_button.update(mouse_position().into());
        self.quit_button.update(mouse_position().into());


        if (self.play_button.hovered || self.quit_button.hovered) && self.should_beep_on_button_hover {
            play_sound(sounds.get(
                "assets/sounds/menu_navigation_beep.wav".into()), 
                PlaySoundParams {
                    looped: false,
                    volume: 0.5,
                }
            );

            self.should_beep_on_button_hover = false


        }

        if !self.play_button.hovered && !self.quit_button.hovered {
            self.should_beep_on_button_hover = true
        }



    }
}