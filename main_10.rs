pub struct AnimationState {
    pub time: f64,
    pub delta: f64,
    pub frame_count: u64,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            delta: 0.0,
            frame_count: 0,
        }
    }

    pub fn update(&mut self, current_time: f64) {
        self.delta = current_time - self.time;
        self.time = current_time;
        self.frame_count += 1;
    }

    pub fn pulse(&self, speed: f32) -> f32 {
        ((self.time as f32 * speed).sin() * 0.5 + 0.5)
    }

    pub fn bounce(&self, speed: f32, amplitude: f32) -> f32 {
        (self.time as f32 * speed).sin().abs() * amplitude
    }

    pub fn wave(&self, speed: f32, offset: f32) -> f32 {
        ((self.time as f32 * speed + offset).sin() * 0.5 + 0.5)
    }
}
