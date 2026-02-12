use std::{
    fs::{self}, io, path::PathBuf, process::exit
};
use clap::Parser;
use image::{GenericImageView, ImageReader};
use interceptors_lib::{background::BackgroundSave, decoration::DecorationSave, prop::{Material, PropSave}};
use macroquad::math::{Vec2, vec2};
use colored::Colorize;
use rapier2d::prelude::RigidBodyType;
use strum::IntoEnumIterator;

use crate::prefab_type::PrefabType;

pub mod prefab_type;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    prefab_type: Option<PrefabType>,
    #[arg(long)]
    scale: Option<f32>,
    /// Input file
    asset_paths: Vec<String>,
    
}

fn get_user_input() -> String {
    // pause execution
    let mut string_input = String::new();
    io::stdin()
        .read_line(&mut string_input)
        .expect("Failed to read line");

    string_input.trim().to_string()
}

fn parse_user_input<T: std::str::FromStr>(prompt: &str, error_message: &str) -> T {
    println!("{prompt}");
    match get_user_input().parse() {
        Ok(value) => value,
        Err(_) => {
            error_print(error_message.to_string());
            exit(1);
        }
    }
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
    
    for asset_path in args.asset_paths {
        asset_to_prefab(asset_path, args.prefab_type, args.scale);
    }

    get_user_input();
    
}

fn asset_to_prefab(asset_path: String, prefab_type: Option<PrefabType>, scale: Option<f32>) {

    let relative_path = match get_path_relative_to_assets_folder(asset_path) {
        Ok(relative_path) => relative_path,
        Err(()) => {
            error_print("Asset must exist in assets directory".to_string());
            return;
        },
    };

    let prefab_type = match prefab_type {
        Some(prefab_type) => prefab_type,
        None => {
            println!("{}", relative_path.to_string_lossy().to_string().bold().underline());
            println!("Select prefab type:");
            println!("({}){}", "D".blue().underline().bold(), "ecoration");
            println!("({}){}", "P".red().underline().bold(), "rop");
            println!("({}){}", "B".green().underline().bold(), "ackground");
            match get_user_input().to_lowercase().as_str() {
                "d" => PrefabType::Decoration,
                "p" => PrefabType::Prop,
                "b" => PrefabType::Background,
                _ => {
                    error_print("Invalid prefab input".to_string());
                    exit(1)
                }

            }
        },
    };
    
    let json_string = match prefab_type {
        PrefabType::Decoration => asset_to_decoration_prefab(&relative_path, scale),
        PrefabType::Prop => asset_to_prop(&relative_path, scale),
        PrefabType::Background => asset_to_background(&relative_path, scale) ,
    }.unwrap();
    
    // removes the parts in brackets
    // [assets]/example_directory/[asset.png]
    let prefab_directory_structure = relative_path
        .parent() 
        .unwrap()
        .components()
        .skip(1)
        .collect::<PathBuf>();

    // add the parts in parenthesis
    // (prefabs)/(prefab_type)/example_directory
    let prefab_save_directory = match prefab_type {
        PrefabType::Decoration => {
            PathBuf::from("prefabs/decorations")
        },
        PrefabType::Prop => {
            PathBuf::from("prefabs/generic_physics_props")
        },
        PrefabType::Background => {
            PathBuf::from("prefabs/backgrounds")
        },
    }.join(prefab_directory_structure);

    // add the parts in parenthesis 
    // prefabs/prefab_type/example_directory/(asset_name).json
    let prefab_save_path = prefab_save_directory
        .join(
            PathBuf::from(
                format!(
                    "{}.json", 
                    relative_path.file_stem().unwrap().to_str().unwrap()
                )
            )
        );
    
    fs::create_dir_all(prefab_save_directory).unwrap();

    println!("{}", &json_string);
    println!("Press enter to save prefab to {}...", &prefab_save_path.to_string_lossy());

    get_user_input();

    fs::write(prefab_save_path, json_string).unwrap();


    
}

fn asset_to_background(relative_path: &PathBuf, scale: Option<f32>) -> Result<String, serde_json::Error> {

    let (width, height) = get_image_dimensions(&relative_path);

    let scale: f32 = match scale {
        Some(scale) => scale, 
        None => {
            loop {
                println!("Enter scaling factor");

                match get_user_input().parse::<f32>() {
                    Ok(scale) => break scale,
                    Err(_error) => {
                        println!("Failed to parse scale input");
                        continue;
                    },
                }
            }
            
        },
    };

    

    let repeat: bool = loop {
        println!("Repeat? (y/n)");
        match get_user_input().as_str() {
            "y" => {
                break true
            },
            "n" => {
                break false
            }
            _ => {
                println!("Invalid input");
                continue;
            }
        }
    };

    let parallax: f32 = loop {
        println!("Enter parallax value: ");

        match get_user_input().parse::<f32>() {
            Ok(parallax) => break parallax,
            Err(_) => {
                println!("Invalid parallax value");
                continue;
            },
        }
    };

    let background_save = BackgroundSave {
        repeat,
        pos: Vec2::ZERO,
        sprite_path: relative_path.clone(),
        size: vec2(width as f32 * scale, height as f32 * scale),
        parallax,
    };

    serde_json::to_string_pretty(&background_save)

    // println!("Background preview:");
    // println!("{}\n", save);
    // println!("Press enter to continue...");
    // get_user_input();

    // let prefab_path = format!("prefabs/backgrounds/{}.json",&relative_path.file_stem().unwrap().to_string_lossy().to_string());


    // match fs::write(&prefab_path, save) {
    //     Ok(_) => todo!(),
    //     Err(error) => {error_print(format!("Error writing destination file '{}': {}", &prefab_path, error));},
    // }

    // log::info!("Successfully wrote prefab to: {}", prefab_path)

    
}   


fn asset_to_prop(relative_path: &PathBuf, scale: Option<f32>) -> Result<String, serde_json::Error> {
    let (width, height) = get_image_dimensions(&relative_path);
    let scale = scale.unwrap_or_else(|| {
        parse_user_input("Enter scaling factor: ", "Failed to parse scaling input")
    });

    let mass: f32 = parse_user_input("Enter mass: ", "Failed to parse mass input");
    println!("Enter name: ");
    let name = get_user_input();

    println!("Select body type: ");
    let body_types = vec![RigidBodyType::Fixed, RigidBodyType::Dynamic];
    for (index, body_type) in body_types.iter().enumerate() {
        println!("({}) {:?}", index, body_type);
    }

    let body_type_selection_index: usize = loop {
        match get_user_input().parse::<usize>() {
            Ok(index) => break index,
            Err(_e) => {
                println!("Failed to parse body type selection index");
                continue;
            },
        }
    };  

    let body_type: &RigidBodyType = loop {

        match body_types
            .get(body_type_selection_index) {
                Some(body_type) => break body_type,
                None => {
                    println!("Invalid body type selection");
                    continue;
                },
            }
    };

    println!("Select material: ");

    for (index, material) in Material::iter().enumerate() {
        let material_string = material.to_string();
        println!("({}) {}", index.to_string().bold(), material_string);
    }
    
    let prop_material_selection_index: usize = loop {
        match get_user_input().parse::<usize>() {
            Ok(index) => break index,
            Err(_e) => {
                println!("Failed to parse prop material selection index");
                continue;
            },
        }
    };  

    let prop_materials = Material::iter()
        .collect::<Vec<Material>>();

    let prop_material: &Material = loop {

        match prop_materials
            .get(prop_material_selection_index) {
                Some(prop_material) => break prop_material,
                None => {
                    println!("Invalid prop material selection");
                    continue;
                },
            }
    };

    
    log::info!("Loaded {} with dimensions: {:?}", &relative_path.to_string_lossy(), (width, height));

    let prop_save = PropSave {
        scale,
        pos: Default::default(),
        mass,
        sprite_path: relative_path.clone(),
        id: None,
        owner: None,
        material: *prop_material,
        name,
        layer: 0,
        voxels: None,
        rigid_body_type: *body_type
    };

    return serde_json::to_string_pretty(&prop_save)

    // println!("{}{} {}{} {}{} {}{}{}{}{}", "P".red(), "rop", "p".green(), "refab", "p".blue(), "review", "(", "p".red(), "p".green(), "p".blue(), ")");
    // println!("{}\n", save);
    // println!("Press enter to continue...");
    // get_user_input();

    // let prefab_path = format!("prefabs/generic_physics_props/{}.json", &relative_path.file_stem().unwrap().to_string_lossy().to_string());

    // match fs::write(&prefab_path, save) {
    //     Ok(_) => {},
    //     Err(error) => error_print(format!("Error writing destination file '{}': {}", &prefab_path, error)),
    // }

    // log::info!("Successfully wrote prefab to: {}", prefab_path);

    
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

fn asset_to_decoration_prefab(asset_path: &PathBuf, scale: Option<f32>) -> Result<String, serde_json::Error> {

    let (width, height) = get_image_dimensions(&asset_path);

    log::info!("Loaded {} with dimensions: {:?}", &asset_path.to_string_lossy(), (width, height));

    let scale: f32 = scale.unwrap_or_else(|| {
        parse_user_input("Enter scaling factor: ", "Failed to parse scaling factor")
    });
    
    let decoration_save = DecorationSave {
        pos: Vec2::ZERO,
        size: Vec2::new(width as f32 * scale, height as f32 * scale),
        sprite_path: asset_path.clone().into(),
        animated_sprite_paths: None,
        frame_duration: None,
        layer: 0,
    };


    return serde_json::to_string_pretty(&decoration_save)


    // we want to copy the directory structure from the assets folder but we gotta get rid of the assets/ part of the path
    // let mut prefab_save_path_structure: PathBuf = asset_path.components().skip(1).collect();

    // prefab_save_path_structure.set_extension("");

    // let prefab_path_string = format!("prefabs/decorations/{}.json", &prefab_save_path_structure.to_string_lossy().to_string());

    // // this is the stupidest thing i've written
    // create_dir_all(format!("prefabs/decorations/{}", prefab_save_path_structure.parent().unwrap().to_str().unwrap().to_string())).unwrap();
    // match fs::write(&prefab_path_string, save) {
    //     Ok(_) => {},
    //     Err(error) => error_print(format!("Error writing destination file '{}': {}", &prefab_path_string, error)),
    // }

    // log::info!("Successfully wrote prefab to: {}", prefab_path_string);

}

fn get_path_relative_to_assets_folder(asset_path: String) -> Result<PathBuf, ()> {
    // get the path relative to assets
    let path = PathBuf::from(asset_path);

    for (i, component) in path.components().enumerate() {
        if component.as_os_str() == "assets" {
            return Ok(path.components().skip(i).collect());
        }
    }

    Err(())
}
