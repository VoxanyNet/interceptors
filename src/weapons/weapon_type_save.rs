use serde::{Deserialize, Serialize};

use crate::weapons::{lmg::weapon_save::LMGSave, shotgun::weapon_save::ShotgunSave};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WeaponTypeSave {
    Shotgun(ShotgunSave),
    LMG(LMGSave)
}
