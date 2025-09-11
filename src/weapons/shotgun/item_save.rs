use serde::{Deserialize, Serialize};

use crate::weapons::weapon::item_save::WeaponItemSave;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShotgunItemSave {
    pub weapon: WeaponItemSave
}