use serde::{Deserialize, Serialize};

use crate::weapons::{lmg::item_save::LMGItemSave, shotgun::item_save::ShotgunItemSave};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WeaponTypeItemSave {
    Shotgun(ShotgunItemSave),
    LMG(LMGItemSave)
}