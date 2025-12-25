use std::{env::{self, current_dir, set_current_dir}, fs, io, path::{Path, PathBuf}, process::exit, time::Duration};

use clap::{Arg, Parser};
use image::{GenericImageView, ImageReader};
use interceptors_lib::{decoration::DecorationSave, prop::{PropMaterial, PropSave}};
use macroquad::{math::Vec2, texture::load_image};
use nalgebra::vector;
use colored::Colorize;
use strum::IntoEnumIterator;

use crate::prefab_type::PrefabType;

pub mod prefab_type;

#[derive(Parser)]
struct Args {
    /// Input file
    asset_paths: Vec<String>
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

    log::info!("Converting {} assets...", args.asset_paths.len());

    //set_current_dir(env::current_exe().unwrap().parent().unwrap()).unwrap();
    
    for asset_path in args.asset_paths {
        asset_to_prefab(asset_path);
    }

    get_user_input();
    
}

fn asset_to_prefab(asset_path: String) {

    let relative_path = match get_path_relative_to_assets_folder(asset_path) {
        Ok(relative_path) => relative_path,
        Err(error) => {
            error_print("Asset must exist in assets directory".to_string());
            return;
        },
    };

    println!("{}", relative_path.to_string_lossy().to_string().bold().underline());
    println!("Select prefab type:");
    println!("({}){}", "D".blue().underline().bold(), "ecoration");
    println!("({}){}", "P".red().underline().bold(), "rop");
    let prefab_type = match get_user_input().to_lowercase().as_str() {
        "d" => PrefabType::Decoration,
        "p" => PrefabType::Prop,
        _ => {
            error_print("Invalid prefab input".to_string());
            exit(1)
        },

    };

    match prefab_type {
        PrefabType::Decoration => asset_to_decoration_prefab(relative_path),
        PrefabType::Prop => asset_to_prop(relative_path),
    }

}

fn asset_to_prop(relative_path: PathBuf) {

    let (width, height) = get_image_dimensions(&relative_path);

    println!("Enter scaling factor: ");
    let scale: f32 = match get_user_input().parse() {
        Ok(scaling_factor) => scaling_factor,
        Err(error) => {
            error_print("Failed to parse scaling input".to_string()); 
            exit(1);
        },
    };

    println!("Enter mass: ");
    let mass: f32 = match get_user_input().parse() {
        Ok(mass) => mass,
        Err(error) => {
            error_print("Failed to parse mass input".to_string()); 
            exit(1);
        },
    };

    println!("Enter name: ");
    let name = get_user_input();

    println!("Select material: ");

    for (index, material) in PropMaterial::iter().enumerate() {
        let material_string = material.to_string();
        println!("({}) {}", index.to_string().bold(), material_string);
    }
    
    let prop_material_selection_index: usize = match get_user_input().parse() {
        Ok(prop_material_selection_index) => prop_material_selection_index,
        Err(error) => {
            error_print("Failed to parse prop material selection index input".to_string()); 
            exit(1);
        },
    };

    // this is a little silly
    let prop_materials: Vec<PropMaterial> = PropMaterial::iter().collect();
    let prop_material = match prop_materials.get(prop_material_selection_index) {
        Some(prop_material) => prop_material,
        None => {
            error_print("Invalid prop material index".to_string());
            exit(1);
        },
    };


    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), (width, height));

    let prop_save = PropSave {
        size: Vec2::new(width as f32 * scale, height as f32 * scale),
        pos: Default::default(),
        mass,
        sprite_path: relative_path.clone(),
        id: None,
        owner: None,
        material: *prop_material,
        name,
    };

    let save = serde_json::to_string_pretty(&prop_save).unwrap();

    println!("{}{} {}{} {}{} {}{}{}{}{}", "P".red(), "rop", "p".green(), "refab", "p".blue(), "review", "(", "p".red(), "p".green(), "p".blue(), ")");
    println!("{}\n", save);
    println!("Press enter to continue...");
    get_user_input();

    let prefab_path = format!("prefabs/generic_physics_props/{}.json", &relative_path.file_stem().unwrap().to_string_lossy().to_string());

    match fs::write(&prefab_path, save) {
        Ok(_) => {},
        Err(error) => error_print(format!("Error writing destination file '{}': {}", &prefab_path, error)),
    }

    log::info!("Successfully wrote prefab to: {}", prefab_path);

    
}

fn get_image_dimensions(asset_path: &PathBuf) -> (u32, u32) {

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

fn asset_to_decoration_prefab(relative_path: PathBuf) {

    let (width, height) = get_image_dimensions(&relative_path);

    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), (width, height));

    println!("Enter scaling factor: ");

    let scale: f32 = match get_user_input().parse() {
        Ok(scaling_factor) => scaling_factor,
        Err(error) => {
            error_print("Failed to parse scaling factor".to_string()); 
            exit(1);
        },
    };

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