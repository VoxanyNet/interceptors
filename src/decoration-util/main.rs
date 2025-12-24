use std::{env::{self, set_current_dir}, fs, io, path::{Path, PathBuf}, time::Duration};

use clap::{Arg, Parser};
use image::{GenericImageView, ImageReader};
use interceptors_lib::decoration::DecorationSave;
use macroquad::{math::Vec2, texture::load_image};



#[derive(Parser)]
struct Args {
    /// Input file
    asset_paths: Vec<String>,

    /// Output file
    #[arg(long)]
    output_path: Option<String>,

    #[arg(long)]
    scale: Option<f32>,
}

fn wait_for_user_enter() {
     // pause execution
    let mut string_input = String::new();
    io::stdin()
        .read_line(&mut string_input)
        .expect("Failed to read line");
}

fn error_print(message: String) {
    log::error!("{}", message);

   wait_for_user_enter();

}

fn main() {

    pretty_env_logger::init();

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            error_print(format!("Parsing args error: {}", error));
            return;
        },
    };

    let scale = match args.scale {
        Some(scale) => scale,
        None => {

            println!("Enter scale factor (1.):");

            let mut string_input = String::new();
            io::stdin()
                .read_line(&mut string_input)
                .expect("Failed to read line");

            let string_input = string_input.trim();

            let scale: f32  = match string_input.parse() {
                Ok(scale) => scale,
                Err(_) => 1.,
            };

            scale
        },
    };

    log::info!("Converting {} assets to prefabs with scaling of {}", args.asset_paths.len(), scale);

    set_current_dir(env::current_exe().unwrap().parent().unwrap()).unwrap();
    
    for asset_path in args.asset_paths {
        asset_to_decoration_prefab(asset_path, scale);
    }

    wait_for_user_enter();
    
}

fn asset_to_decoration_prefab(asset_path: String, scale: f32) {

    let img = match ImageReader::open(&asset_path) {
        Ok(image_reader) => {
            match image_reader.decode() {
                Ok(img) => img,
                Err(error) => {
                    error_print(format!("Image decoding error: {}", error));
                    
                    return;
                },
            }
        },
        Err(error) => {
            error_print(format!("Image opening error: {}", error));

            return;
        },
    };

    

    let (width, height) = img.dimensions();

    

    let relative_path = match get_path_relative_to_assets_folder(asset_path) {
        Ok(relative_path) => relative_path,
        Err(error) => {
            error_print("Asset must exist in assets directory".to_string());
            return;
        },
    };

    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), img.dimensions());

    let decoration_save = DecorationSave {
        pos: Vec2::ZERO,
        size: Vec2::new(width as f32 * scale, height as f32 * scale),
        sprite_path: relative_path.clone().into(),
        animated_sprite_paths: None,
        frame_duration: None,
        layer: 0,
    };

    

    let save = serde_json::to_string_pretty(&decoration_save).unwrap();


    let prefab_path = format!("prefabs/decorations/{}.json", &relative_path.file_stem().unwrap().to_string_lossy().to_string());

    match fs::write(&prefab_path, save) {
        Ok(_) => {},
        Err(error) => error_print(format!("Error writing destination file '{}': {}", &prefab_path, error)),
    }

    log::info!("Successfully wrote prefab to: {}", prefab_path);

}

fn get_path_relative_to_assets_folder(asset_path: String) -> Result<PathBuf, ()> {
    // get the path relative to assets
    let path = PathBuf::from(asset_path);

    for (i, component) in path.components().enumerate() {
        if component.as_os_str() == "assets" {
            return Result::Ok(path.components().skip(i).collect())

        }
    };

    return Result::Err(())
}