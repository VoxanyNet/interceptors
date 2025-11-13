use interceptors_lib::prop::PropId;
use nalgebra::Vector2;

#[derive(Clone, PartialEq)]
pub enum SelectableObjectId {
    Decoration(usize),
    Tile(Vector2<usize>),
    Prop(PropId)
}