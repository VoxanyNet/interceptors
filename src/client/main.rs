
use std::{path::PathBuf, time::Instant};

use activity::DiscordSDK;
use include_dir::include_dir;
use interceptors_lib::{Prefabs, font_loader::FontLoader, load_assets, sound_loader::SoundLoader, texture_loader::TextureLoader};
use macroquad::{input::show_mouse, miniquad::{conf::Platform, window::request_quit}, window::Conf};

use crate::{client::{Client}, main_menu::{MainMenu, MainMenuResult}};

mod client;
mod main_menu;
mod shaders;


fn window_conf() -> Conf {
    let conf = Conf {
        window_title: "Interceptors".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        fullscreen: false, 
        platform: Platform::default(),
        ..Default::default()
    };
    //conf.platform.swap_interval = Some(0); // disable vsync
    conf
}
#[macroquad::main(window_conf)]
async fn main() {

    pretty_env_logger::init();  

    let client_id: i64 = 1461559630462451868;

    #[cfg(feature = "discord")] {
        
        let sdk = DiscordSDK::new(&client_id.to_string()).unwrap();

        sdk.ready().await.unwrap();
    }
    
    let assets = load_assets().await;

    
    let mut main_menu = MainMenu::new(assets.clone()).await;

    show_mouse(false);

    match main_menu.run().await {
        MainMenuResult::Quit => {
            request_quit();
        },
        MainMenuResult::Connect => {
            let mut client = Client::connect(assets, client_id).await;

            client.run().await;
        },
    }
    
    
}





