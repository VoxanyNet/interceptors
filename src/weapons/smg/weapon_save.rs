use serde::{Deserialize, Serialize};

use crate::weapons::weapon::weapon_save::WeaponSave;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SMGSave {
    pub weapon_base: WeaponSave
}