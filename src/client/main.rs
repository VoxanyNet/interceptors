use std::io::read_to_string;

use ewebsock::{WsReceiver, WsSender};
use ldtk2::{serde_json, Ldtk};
use macroquad::{miniquad::conf::Platform, window::{next_frame, Conf}};

use crate::client::Client;

mod client;



fn window_conf() -> Conf {
    let mut conf = Conf {
        window_title: "Interceptors".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        fullscreen: false, 
        platform: Platform::default(),
        ..Default::default()
    };
    conf.platform.swap_interval = Some(0); // disable vsync
    conf
}
#[macroquad::main(window_conf)]
async fn main() {
    
    let mut client = Client::connect().await;

    client.run().await;
}

