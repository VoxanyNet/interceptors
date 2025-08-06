use macroquad::{math::Vec2, window::get_internal_gl};
use serde::{Deserialize, Serialize};

use crate::{prop::{Prop, PropSave}, space::Space, ClientTickContext};

// equivalent to chunk in minecraft
pub struct Area {
    id: u32,
    spawn_point: Vec2,
    space: Space,
    props: Vec<Prop>,
}

impl Area {
    pub fn empty() -> Self {

        
        Self {
            spawn_point: Vec2::ZERO,
            space: Space::new(),
            props: Vec::new(),
            id: uuid::Uuid::new_v4().as_u64_pair().0 as u32,
        }
    }

    pub fn server_tick(&mut self) {

    }

    pub fn client_tick(&mut self, ctx: &mut ClientTickContext) {
        for prop in &mut self.props {
            prop.client_tick(&mut self.space);
        }
    }

    pub fn from_save(save: AreaSave) -> Self {

        let mut space = Space::new();

        let mut props: Vec<Prop> = Vec::new();

        for prop_save in save.props {
            let prop = Prop::from_save(prop_save, &mut space);

            props.push(prop);
        }

        Self {
            spawn_point: save.spawn_point,
            space,
            props,
            id: save.id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AreaSave {
    id: u32,
    spawn_point: Vec2,
    props: Vec<PropSave>,
    offset: Vec2
}