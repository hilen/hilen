use web_time::Instant;

pub(crate) struct FrameCounter {
    last_frame_update: Instant,
    last_fps_update:   Instant,

    pub(crate) fps:         f32,
    pub(crate) frame_time:  f32,
    pub(crate) frame_count: u32,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            last_frame_update: Instant::now(),
            last_fps_update:   Instant::now(),

            fps:         0.0,
            frame_time:  0.0,
            frame_count: 0,
        }
    }
}

impl FrameCounter {
    pub(crate) fn update(&mut self) -> bool {
        self.frame_count += 1;
        let now = Instant::now();

        self.frame_time = (now - self.last_frame_update).as_secs_f32();
        self.fps = 1.0 / self.frame_time;
        self.last_frame_update = now;

        let passed = (now - self.last_fps_update).as_secs_f32();

        if passed < 1.0 {
            return false;
        }

        self.last_fps_update = now;

        true
    }
}
