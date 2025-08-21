use std::time::Instant;

use nalgebra::Isometry2;

use crate::{prop::{Prop, PropSave}, texture_loader::TextureLoader, Prefabs};

pub struct Computer {
    prop: Prop,
}

impl Computer {
    pub fn new(prefabs: Prefabs, space:&mut crate::space::Space, pos: Isometry2<f32> ) -> Self {
        
        let save: PropSave = serde_json::from_str(
            &prefabs.get_prefab_data("prefabs/computer.json")
        ).unwrap();

        let mut prop = Prop::from_save(
            save, 
            space
        );

        prop.set_pos(pos, space);

        Self {
            prop,
        }
    }
    
    pub async fn draw(&self, textures: &mut TextureLoader, space:&crate::space::Space) {
        self.prop.draw(space, textures).await;

    }
}