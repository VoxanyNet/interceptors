use std::{env::{self, set_current_dir}, fs, io, path::{Path, PathBuf}, process::exit, time::Duration};

use clap::{Arg, Parser};
use image::{GenericImageView, ImageReader};
use interceptors_lib::{decoration::DecorationSave, prop::{PropMaterial, PropSave}};
use macroquad::{math::Vec2, texture::load_image};
use nalgebra::vector;

use crate::prefab_type::PrefabType;

pub mod prefab_type;

#[derive(Parser)]
struct Args {
    /// Input file
    asset_paths: Vec<String>,

    #[arg(long)]
    scale: Option<f32>,
}

fn get_user_input() -> String {
     // pause execution
    let mut string_input = String::new();
    io::stdin()
        .read_line(&mut string_input)
        .expect("Failed to read line");

    string_input.trim().to_string()
}   

fn error_print(message: String) {
    log::error!("{}", message);

   get_user_input();

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
        asset_to_prefab(asset_path);
    }

    get_user_input();
    
}

fn asset_to_prefab(asset_path: String) {

    let relative_path = match get_path_relative_to_assets_folder(relative_path) {
        Ok(relative_path) => relative_path,
        Err(error) => {
            error_print("Asset must exist in assets directory".to_string());
            return;
        },
    };

    println!("")
}

fn asset_to_prop(relative_path: PathBuf, scale: f32, mass: f32, material: PropMaterial, name: String) {

    let (width, height) = get_image_dimensions(&relative_path);

    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), (width, height));

    let prop_save = PropSave {
        size: Vec2::new(width as f32 * scale, width as f32 * scale),
        pos: Default::default(),
        mass,
        sprite_path: relative_path.clone(),
        id: None,
        owner: None,
        material: material,
        name,
    };

    let save = serde_json::to_string_pretty(&prop_save).unwrap();

    let prefab_path = format!("prefabs/decorations/{}.json", &relative_path.file_stem().unwrap().to_string_lossy().to_string());

    match fs::write(&prefab_path, save) {
        Ok(_) => {},
        Err(error) => error_print(format!("Error writing destination file '{}': {}", &prefab_path, error)),
    }

    log::info!("Successfully wrote prefab to: {}", prefab_path);

    
}

fn get_image_dimensions(asset_path: &str) -> (u32, u32) {

    let img = match ImageReader::open(&asset_path) {
        Ok(image_reader) => {
            match image_reader.decode() {
                Ok(img) => img,
                Err(error) => {
                    error_print(format!("Image decoding error: {}", error));
                    
                    exit(1);
                },
            }
        },
        Err(error) => {
            error_print(format!("Image opening error: {}", error));

            exit(1)
        },
    };

    

    img.dimensions()

}

fn asset_to_decoration_prefab(relative_path: PathBuf, scale: f32) {

    let (width, height) = get_image_dimensions(&asset_path);

    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), (width, height));

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