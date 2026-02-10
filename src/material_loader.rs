use std::{fmt::Debug, path::{Path, PathBuf}};

use fxhash::FxHashMap;
use macroquad::prelude::{Material, MaterialParams, ShaderSource, UniformDesc, load_material};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MaterialLoader {
    materials: FxHashMap<PathBuf, Material>
}

impl MaterialLoader {
    pub fn new() -> Self {
        Self {
            materials: FxHashMap::default(),
        }
    }

    pub fn load(
        &mut self, 
        material_definition_path: PathBuf, 
        material_meta: MaterialMeta,
        vertex: String,
        fragment: String
    ) {

        let mut uniforms = Vec::new();

        for uniform in material_meta.uniforms {
            uniforms.push(
                UniformDesc {
                    name: uniform.name,
                    uniform_type: uniform.uniform_type.into(),
                    array_count: 1,
                }
            );
        }
        let material_params = MaterialParams {            
            uniforms: uniforms,
            textures: material_meta.textures,
            ..Default::default()
        };
        
        let material = load_material(
            ShaderSource::Glsl { 
                vertex: &vertex, 
                fragment: &fragment 
            }, 
            material_params
        ).unwrap();
        
        self.materials.insert(material_definition_path, material);
    }

    pub fn get<P: AsRef<Path> + Debug>(&self, path: P) -> &Material {
 
        self.materials.get(path.as_ref()).expect("Material not found")
    }   

}

/// The data we need to give to load_material so that it knows what uniforms and textures it has
#[derive(Serialize, Deserialize, Clone)]
pub struct MaterialMeta {
    textures: Vec<String>,
    uniforms: Vec<Uniform>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uniform {
    name: String,
    uniform_type: UniformType
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum UniformType {
    Float1,
    Float2,
}

impl Into<macroquad::prelude::UniformType> for UniformType {
    fn into(self) -> macroquad::prelude::UniformType {
        match self {
            UniformType::Float1 => macroquad::prelude::UniformType::Float1,
            UniformType::Float2 => macroquad::prelude::UniformType::Float2,
        }
    }
}
