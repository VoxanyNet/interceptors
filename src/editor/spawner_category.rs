use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, Clone, Copy)]
pub enum SpawnerCategory {
    Decoration,
    Background,
    Prop,
    Tile
}
