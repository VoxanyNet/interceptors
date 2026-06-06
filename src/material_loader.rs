use std::{collections::HashMap, fmt::Debug, path::{Path, PathBuf}};

use fxhash::FxHashMap;
use macroquad::prelude::{Material, MaterialParams, ShaderSource, UniformDesc, load_material};
use serde::{Deserialize, Serialize};

use crate::uuid_u64;


#[derive(Debug, Hash, PartialEq, PartialOrd, Clone, Eq )]
pub struct ExclusiveMaterialHandle {
    id: u32,
    path: PathBuf
}

impl ExclusiveMaterialHandle {
    pub fn new<P: Into<PathBuf> + Debug>(path: P) -> Self {
        Self {
            id: uuid_u64() as u32,
            path: path.into(),
        }
    }
}
#[derive(Clone)]
pub struct MaterialLoader {
    // we load these materials when the game starts, and are shared between multiple entities
    pub shared_materials: FxHashMap<PathBuf, Material>,
    // this materials are loaded during gameplay and are only used by one entity
    pub exlusive_materials: FxHashMap<ExclusiveMaterialHandle, Material>,
    // store the material definitions so we can load new copies later
    // we need this because the base prop entity requires unique materials for each entity to prevent batching
    pub material_definitions: FxHashMap<PathBuf, MaterialDefinition>
}

impl MaterialLoader {
    pub fn new() -> Self {
        Self {
            shared_materials: FxHashMap::default(),
            exlusive_materials: FxHashMap::default(),
            material_definitions: FxHashMap::default()
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

        for uniform in material_meta.uniforms.clone() {
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
            textures: material_meta.textures.clone(),
            ..Default::default()
        };
        
        let material = load_material(
            ShaderSource::Glsl { 
                vertex: &vertex, 
                fragment: &fragment 
            }, 
            material_params
        ).unwrap();

        // store all the material information for later additional loads
        self.material_definitions.insert(
            material_definition_path.clone(), 
            MaterialDefinition {
                vertex,
                fragment,
                meta: material_meta,
            }
        );

        self.shared_materials.insert(material_definition_path, material);
    }

    pub fn get<P: AsRef<Path> + Debug>(&self, path: P) -> Material {
 
        self.shared_materials.get(path.as_ref()).expect("Material not found").clone()

    }

    pub fn get_exclusive(
        &mut self, handle: &ExclusiveMaterialHandle
    ) -> Material { 

        if !self.exlusive_materials.contains_key(handle) {
            let material_definition = self.material_definitions.get(&handle.path).unwrap();

            let mut uniforms = Vec::new();

            for uniform in material_definition.meta.uniforms.clone() {
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
                textures: material_definition.meta.textures.clone(),
                ..Default::default()
            };
            
            let material = load_material(
                ShaderSource::Glsl { 
                    vertex: &material_definition.vertex, 
                    fragment: &material_definition.fragment 
                }, 
                material_params
            ).unwrap();

            self.exlusive_materials.insert(handle.clone(), material);
        }

        

        self.exlusive_materials.get(handle).unwrap().clone()
    }

}

#[derive(Clone)]
// all the data we need to load a material
struct MaterialDefinition {
    vertex: String,
    fragment: String,
    meta: MaterialMeta

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
