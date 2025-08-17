pub struct ScreenShakeParameters {
    pub x_intensity: f64,
    pub x_frequency: f64,
    pub x_offset: f64,
    pub x_noise: Perlin,
    pub x_intensity_decay: f64,
    pub x_frequency_decay: f64,

    pub y_intensity: f64,
    pub y_frequency: f64,
    pub y_offset: f64,
    pub y_noise: Perlin,
    pub y_intensity_decay: f64,
    pub y_frequency_decay: f64

}