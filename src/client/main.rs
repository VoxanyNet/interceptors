
use std::{path::PathBuf, time::Instant};

use interceptors_lib::{Prefabs, font_loader::FontLoader, sound_loader::SoundLoader, texture_loader::TextureLoader};
use macroquad::{miniquad::{conf::Platform, window::request_quit}, window::Conf};

use crate::{client::{ASSET_PATHS, Client, PREFAB_PATHS}, main_menu::{MainMenu, MainMenuResult}};

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

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let assets = load_assets().await;

    let mut main_menu = MainMenu::new(assets.clone()).await;

    match main_menu.run().await {
        MainMenuResult::Quit => {
            request_quit();
        },
        MainMenuResult::Connect => {
            let mut client = Client::connect(assets).await;

            client.run().await;
        },
    }
    
    
}


#[derive(Clone)]
/// Safe to clone because these are all handles
pub struct Assets {
    sounds: SoundLoader,
    fonts: FontLoader,
    textures: TextureLoader,
    prefabs: Prefabs
}
async fn load_assets() -> Assets {
    let mut sounds = SoundLoader::new();
    let mut fonts = FontLoader::new();
    let mut textures = TextureLoader::new();
    let mut prefabs = Prefabs::new();

    for prefab_path in PREFAB_PATHS {
        prefabs.load_prefab_data(prefab_path).await
    }

    for asset in ASSET_PATHS {
        if asset.ends_with(".wav") {
            sounds.load(PathBuf::from(asset)).await
        }

        if asset.ends_with(".png") {
            textures.load(PathBuf::from(asset)).await;
        }

        if asset.ends_with(".ttf") {
            fonts.load(PathBuf::from(asset)).await;
        }

    }

    Assets {
        sounds,
        fonts,
        textures,
        prefabs,
    }

}


