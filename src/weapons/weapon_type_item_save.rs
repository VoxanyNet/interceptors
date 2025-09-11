use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WeaponTypeItemSave {
    Shotgun(ShotgunItemSave),
    LMG(LMGItemSave)
}