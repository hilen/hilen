/// A fade of a node running toward `to`, see `NodeTemplates::fade_to`.
/// It moves with the scene's update steps, the clock the clips keep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FadeRun {
    pub(crate) to:         f32,
    /// Fade per scene second, always positive.
    pub(crate) per_second: f32,
}

impl FadeRun {
    /// A run from `from` to `to` over `seconds`, none when there is
    /// nothing to run.
    pub(crate) fn new(from: f32, to: f32, seconds: f32) -> Option<Self> {
        (seconds > 0.0 && (to - from).abs() > f32::EPSILON).then(|| Self {
            to,
            per_second: (to - from).abs() / seconds,
        })
    }

    /// The fade `dt` seconds after `fade`, and whether the run is over.
    pub(crate) fn step(self, fade: f32, dt: f32) -> (f32, bool) {
        let left = self.to - fade;
        let step = self.per_second * dt;
        if left.abs() <= step {
            (self.to, true)
        } else {
            (fade + step.copysign(left), false)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_fade_in_reaches_its_end_on_time_and_stops_there() {
        let run = FadeRun::new(0.0, 1.0, 0.3).expect("a run");
        let (mut fade, mut done) = (0.0, false);
        let mut steps = 0;
        while !done {
            (fade, done) = run.step(fade, 0.1);
            steps += 1;
        }
        assert_eq!(fade, 1.0);
        assert_eq!(steps, 3);
    }

    #[test]
    fn a_fade_out_runs_down() {
        let run = FadeRun::new(1.0, 0.25, 1.5).expect("a run");
        let (fade, done) = run.step(1.0, 0.5);
        assert!((fade - 0.75).abs() < 1e-6);
        assert!(!done);
        assert_eq!(run.step(0.3, 0.5), (0.25, true));
    }

    #[test]
    fn no_run_without_time_or_change() {
        assert_eq!(FadeRun::new(0.0, 1.0, 0.0), None);
        assert_eq!(FadeRun::new(0.5, 0.5, 1.0), None);
    }
}
