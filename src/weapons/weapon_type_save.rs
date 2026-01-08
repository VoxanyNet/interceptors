use serde::{Deserialize, Serialize};

use crate::weapons::{lmg::weapon_save::LMGSave, shotgun::weapon_save::ShotgunSave, smg::weapon_save::SMGSave};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WeaponTypeSave {
    Shotgun(ShotgunSave),
    LMG(LMGSave),
    SMG(SMGSave)
}
