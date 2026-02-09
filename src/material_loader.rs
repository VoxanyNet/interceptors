use std::path::PathBuf;

use fxhash::FxHashMap;
use macroquad::prelude::{Material, MaterialParams, ShaderSource, load_material};
use serde::{Deserialize, Serialize};

pub struct MaterialLoader {
    materials: FxHashMap<PathBuf, Material>
}

impl MaterialLoader {
    pub fn new() -> Self {
        Self {
            materials: FxHashMap::default(),
        }
    }

    pub fn load(&mut self, material_definition_path: PathBuf, shader_source: ShaderSource, material_params: MaterialParams) {
        let material = load_material(shader_source, material_params).unwrap();

        self.materials.insert(material_definition_path, material);
    }

    pub fn get(&self, material_definition_path: &PathBuf) -> &Material {
        self.materials.get(material_definition_path).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MaterialDefinition {
    vertex: String,
    fragment: String,
    mat

}