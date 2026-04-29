

use interceptors_lib::load_assets;
use macroquad::{input::show_mouse, miniquad::{conf::Platform, window::request_quit}, window::Conf};
use wasm_logger::Config;

use crate::{client::{Client}, main_menu::{MainMenu, MainMenuResult}};


mod client;
mod main_menu;
mod shaders;

#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}


fn window_conf() -> Conf {
    let mut conf = Conf {
        window_title: "Interceptors".to_owned(),
        window_width: 900,
        window_height: 900,
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

    #[cfg(target_family = "wasm")]
    unsafe {
        __wasm_call_ctors();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::init();

    



    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(Config::default());

    #[cfg(feature = "discord")] {
        let client_id: i64 = 1461559630462451868;
        let sdk = DiscordSDK::new(&client_id.to_string()).unwrap();
        sdk.ready().await.unwrap();
    }

    let assets = load_assets().await;
    let mut main_menu = MainMenu::new(assets.clone()).await;

    show_mouse(false);

    let mut client = Client::connect(assets).await;

    client.run().await;

    return;

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
