use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{TextureLoader, base_prop::Material, base_prop_save::BasePropSave, prop::Prop, prop_save::PropSave, props::wooden_box::wooden_box::WoodenBox, space::Space};

// For props that inherit from base prop save we just keep the full base prop save
// keeping the entire base prop save is a bit wasteful because we overwrite their values with constants when we load
// but its worth it because there is less to maintain and update when the base prop changes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WoodenBoxSave {
    base_prop_save: BasePropSave 
}

#[typetag::serde]
impl PropSave for WoodenBoxSave {
    fn load(&self, space: &mut Space,textures:TextureLoader) -> Box<dyn Prop>  {
        
        // we override the loaded data with constants that define this 'subclass' of BaseProp
        // this is what really makes the subclasses different - how this load function is implemented and what overrides it does
        // this is why i changed the way props are saved so drastically - i wanted to be able to update the props in code without having to update the area save files


        // this is required until i change self to be mutable
        let mut base_prop_save = self.base_prop_save.clone();

        base_prop_save.sprite_path = PathBuf::from_str("assets/box2.png").unwrap();
        base_prop_save.mass = 5.;
        base_prop_save.material = Material::Wood;
        base_prop_save.name = "Crate".to_string();

        let wooden_box = WoodenBox {
            base_prop: base_prop_save.inner_load(space, textures)
        };

        Box::new(
            wooden_box
        )
    }

}
