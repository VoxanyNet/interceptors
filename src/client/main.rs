
use macroquad::{miniquad::conf::Platform, window::Conf};

use crate::client::Client;

mod client;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    let mut client = Client::connect().await;

    client.run().await;
}

