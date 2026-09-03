/// A clip of a node's model playing, see `NodeTemplates::play`. The
/// time moves with the scene's update steps, the same clock the physics
/// keep, so a stepped test lands it on the same frame every run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    pub(crate) clip: usize,
    /// Seconds into the clip.
    pub(crate) time: f32,
    /// Clip seconds per scene second, 1 as authored.
    pub speed:       f32,
    /// Wrap at the end, else hold the last frame.
    pub looped:      bool,
}

impl Playback {
    pub(crate) fn new(clip: usize, looped: bool) -> Self {
        Self {
            clip,
            time: 0.0,
            speed: 1.0,
            looped,
        }
    }

    pub(crate) fn advance(&mut self, dt: f32, duration: f32) {
        self.time += dt * self.speed;
        if self.looped && duration > 0.0 {
            self.time = self.time.rem_euclid(duration);
        } else {
            self.time = self.time.clamp(0.0, duration);
        }
    }

    /// Jumps to `time` seconds into the clip, held inside it.
    pub(crate) fn seek(&mut self, time: f32, duration: f32) {
        self.time = time.clamp(0.0, duration);
    }

    pub(crate) fn finished(&self, duration: f32) -> bool {
        !self.looped && self.time >= duration
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_loop_wraps_and_a_single_run_holds_the_end() {
        let mut looped = Playback::new(0, true);
        looped.advance(1.5, 1.0);
        assert!((looped.time - 0.5).abs() < 1e-6);
        assert!(!looped.finished(1.0));

        let mut once = Playback::new(0, false);
        once.advance(0.4, 1.0);
        assert!(!once.finished(1.0));
        once.advance(2.0, 1.0);
        assert!((once.time - 1.0).abs() < 1e-6);
        assert!(once.finished(1.0));
    }

    #[test]
    fn a_seek_stays_inside_the_clip_and_a_zero_speed_holds_it() {
        let mut playback = Playback::new(0, true);
        playback.seek(5.0, 2.0);
        assert!((playback.time - 2.0).abs() < 1e-6);
        playback.seek(0.75, 2.0);
        playback.speed = 0.0;
        playback.advance(1.0, 2.0);
        assert!((playback.time - 0.75).abs() < 1e-6);
    }

    #[test]
    fn speed_scales_the_step_and_a_reverse_loop_stays_in_range() {
        let mut playback = Playback::new(0, true);
        playback.speed = -2.0;
        playback.advance(0.25, 1.0);
        assert!((playback.time - 0.5).abs() < 1e-6);
    }
}
