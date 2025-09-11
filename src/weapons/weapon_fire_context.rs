pub struct WeaponFireContext<'a> {
    pub space: &'a mut Space,
    pub players: &'a mut Vec<Player>,
    pub props: &'a mut Vec<Prop>,
    pub bullet_trails: &'a mut Vec<BulletTrail>,
    pub facing: Facing,
    pub area_id: AreaId,
    pub dissolved_pixels: &'a mut Vec<DissolvedPixel>,
    pub enemies: &'a mut Vec<Enemy>,
    pub weapon_owner: WeaponOwner
}