use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let asset_dir = Path::new("assets");
    let prefab_dir = Path::new("prefabs");

    let mut asset_paths = Vec::new();
    let mut prefabs_paths: Vec<String> = Vec::new();

    fn collect_files(dir: &Path, base: &Path, paths: &mut Vec<String>, valid_extensions: &Vec<&str> ) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {

                if let Some(extension) = path.extension() {

                    for valid_extension in valid_extensions {
                        if extension.eq_ignore_ascii_case(valid_extension) {
                            let string_path = path.to_string_lossy();
                            paths.push(string_path.into_owned());
                        }
                    }
                    
                }
                
            } else if path.is_dir() {
                collect_files(&path, base, paths, valid_extensions);
            }
        }
    }

    collect_files(&asset_dir, &asset_dir, &mut asset_paths, &vec!["png", "wav", "ttf"]);
    collect_files(&prefab_dir, &prefab_dir, &mut prefabs_paths, &vec!["json"]);

    let contents = format!(
        "pub static ASSET_PATHS: &[&str] = &{:?};",
        asset_paths
    );

    let prefab_contents = format!(
        "pub static PREFAB_PATHS: &[&str] = &{:?};",
        prefabs_paths
    );

    

    fs::write(Path::new(&out_dir).join("assets.rs"), contents).unwrap();
    fs::write(Path::new(&out_dir).join("prefabs.rs"), prefab_contents).unwrap();
}
