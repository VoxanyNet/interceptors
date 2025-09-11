#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeaponItemSave {
    pub mass: f32,
    pub texture_size: Vec2,
    pub sprite: PathBuf,
    pub scale: f32,
    pub fire_sound_path: PathBuf,
    pub x_screen_shake_frequency: f64,
    pub x_screen_shake_intensity: f64,
    pub y_screen_shake_frequency: f64,
    pub y_screen_shake_intensity: f64,
    pub shell_sprite: Option<String>,
    pub rounds: u32,
    pub capacity: u32,
    pub reserve_capacity: u32,
    pub reload_duration: f32,
    pub base_damage: f32,
    pub knockback: f32
}