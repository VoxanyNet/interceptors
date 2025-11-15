use interceptors_lib::{area::Area, decoration::Decoration, prop::{Prop, PropId}, tile::Tile};
use nalgebra::Vector2;
use serde::de;

#[derive(Clone, PartialEq)]
pub enum SelectableObjectId {
    Decoration(usize),
    Tile(Vector2<usize>),
    Prop(PropId)
}

pub enum SelectableObject<'a> {
    Decoration(&'a mut Decoration),
    Tile(&'a mut Tile),
    Prop(&'a mut Prop)
}

impl SelectableObjectId {
    pub fn get_object<'a> (&self, props: &'a mut Vec<Prop>, tiles: &'a mut Vec<Vec<Option<Tile>>>, decorations: &'a mut Vec<Decoration>) -> Option<SelectableObject<'a>> {
        match self {
            SelectableObjectId::Decoration(decoration_index) => {
                if let Some(decoration) = decorations.get_mut(*decoration_index) {
                    Some(
                        SelectableObject::Decoration(decoration)
                    )
                } else {
                    None
                }
            },
            SelectableObjectId::Tile(location) => {
                None
            },
            SelectableObjectId::Prop(prop_id) => {
                if let Some(prop) = props.iter_mut().find(|prop| {prop.id == *prop_id}) {
                    Some(SelectableObject::Prop(prop))
                } else {
                    None
                }
            },
        }
    }
}   