use noise::Perlin;

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

impl ScreenShakeParameters {
    pub fn default(x_seed: Option<u32>, y_seed: Option<u32>) -> Self {

        let x_seed = x_seed.unwrap_or_else(|| 69420);

        let y_seed = y_seed.unwrap_or_else(|| 42069);


        Self {
            x_intensity: 0.,
            x_frequency: 0.,
            x_offset: 0.,
            x_noise: Perlin::new(x_seed),
            x_intensity_decay: 0.,
            x_frequency_decay: 0.,

            y_intensity: 0.,
            y_frequency: 0.,
            y_offset: 0.,
            y_noise: Perlin::new(y_seed),
            y_intensity_decay: 0.,
            y_frequency_decay: 0.,
        }
    }
}