use crate::{TickContext, area::{AreaContext, AreaId}, bullet_trail::BulletTrail, dissolved_pixel::DissolvedPixel, enemy::Enemy, player::{Facing, Player}, prop::Prop, space::Space, weapons::weapon::weapon::WeaponOwner};

pub struct WeaponFireContext<> {
    pub facing: Facing,
    pub weapon_owner: WeaponOwner,
}