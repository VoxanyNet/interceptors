use crate::{clip::Clip, decoration::Decoration, drawable::Drawable, prop::{Prop, PropId, PropTrait}, tile::Tile};
use nalgebra::Vector2;

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum SelectableObjectId {
    Decoration(usize), // index into vec
    Tile(Vector2<usize>), // coordinates
    Prop(PropId),
    Clip(usize) // index into vec
}

pub enum SelectableObject<'a> {
    Decoration(&'a mut Decoration),
    Tile(&'a mut Tile),
    Prop(&'a mut Box<dyn PropTrait>),
    Clip(&'a mut Clip)
}

impl<'a> SelectableObject<'a> {
    pub fn get_layer(&self) -> u32 {
        match self {
            SelectableObject::Decoration(decoration) => decoration.draw_layer(),
            SelectableObject::Tile(tile) => 0,
            SelectableObject::Prop(prop) => prop.draw_layer(),
            SelectableObject::Clip(clip) => clip.layer
        }
    }
}

impl SelectableObjectId {

    pub fn get_object<'a> (&self, 
        props: &'a mut Vec<Box<dyn PropTrait>>, 
        tiles: &'a mut Vec<Vec<Option<Tile>>>, 
        decorations: &'a mut Vec<Decoration>,
        clips: &'a mut Vec<Clip>
    ) -> Option<SelectableObject<'a>> {
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
                if let Some(prop) = props.iter_mut().find(|prop| {prop.as_prop().id == *prop_id}) {
                    Some(SelectableObject::Prop(prop))
                } else {
                    None
                }
            },
            SelectableObjectId::Clip(clip_id) => {
                if let Some(clip) = clips.get_mut(*clip_id) {
                    Some(SelectableObject::Clip(clip))
                } else {
                    None
                }
            }
        }
    }
}   