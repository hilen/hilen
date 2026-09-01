use crate::gm::{Clock, LossyConvert, ToF32};

const SEC: f32 = 1_000.0;

#[derive(Default, Debug)]
pub struct Animation {
    start:    f32,
    span:     f32,
    duration: f32,
    /// Engine clock milliseconds at creation, wall clock in a normal run and
    /// virtual time in a stepped test. All time math measures against it.
    stamp:    f64,
}

impl Animation {
    pub fn new(start: impl ToF32, end: impl ToF32, duration: impl ToF32) -> Self {
        let start = start.to_f32() * SEC;
        let end = end.to_f32() * SEC;
        let span = end - start;
        assert_ne!(span.to_bits(), 0);
        Self {
            start,
            span,
            duration: duration.to_f32() * SEC,
            stamp: Clock::now_ms(),
        }
    }

    /// A default animation has no duration, its value would divide by zero.
    pub(crate) fn is_empty(&self) -> bool {
        self.duration.to_bits() == 0
    }

    pub(crate) fn finished(&self) -> bool {
        self.finished_at(Clock::now_ms())
    }

    pub(crate) fn active(&self) -> bool {
        !self.finished()
    }

    pub fn value(&self) -> f32 {
        self.value_at(Clock::now_ms())
    }

    fn finished_at(&self, now: f64) -> bool {
        now >= self.stamp + f64::from(self.duration)
    }

    /// The sampled value at a given clock time. `value` reads the engine clock,
    /// a test can pass an explicit time to check the curve without a real wait.
    fn value_at(&self, now: f64) -> f32 {
        let delta: f32 = (now - self.stamp).lossy_convert();
        let passed: u64 = (delta / self.duration).lossy_convert();
        let even = passed.is_multiple_of(2);
        let passed: f32 = passed.lossy_convert();
        let delta = delta - (passed * self.duration);
        let ratio = delta / (self.duration);
        let span = if even {
            self.span * ratio
        } else {
            self.span - self.span * ratio
        };
        (self.start + span) / SEC
    }
}

#[cfg(test)]
mod test {
    use std::{thread::sleep, time::Duration};

    use crate::gm::Animation;

    // Deterministic, no wall clock. Proves the curve directly at chosen clock
    // times, which is exactly what frame stepped mode feeds it in a test.
    #[test]
    fn value_at_exact() {
        let anim = Animation::new(0.0, 1.0, 0.5);
        let start = anim.stamp;

        assert!(anim.value_at(start).abs() < 1e-6);
        assert!(!anim.finished_at(start));

        assert!((anim.value_at(start + 250.0) - 0.5).abs() < 1e-4);
        assert!(!anim.finished_at(start + 250.0));

        assert!((anim.value_at(start + 500.0) - 1.0).abs() < 1e-4);
        assert!(anim.finished_at(start + 500.0));

        // Past the duration it bounces back down on the odd pass.
        assert!((anim.value_at(start + 750.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    #[ignore = "flaky, sleep based timing"]
    fn test() {
        let anim = Animation::new(0.0, 1.0, 0.5);

        assert!(
            anim.value() >= 0.0 && anim.value() <= 0.002,
            "Actual: {}",
            anim.value()
        );
        assert!(!anim.finished());

        sleep(Duration::from_secs_f32(0.25));

        assert!(!anim.finished());
        assert!(
            anim.value() >= 0.48 && anim.value() <= 0.52,
            "Actual: {}",
            anim.value()
        );

        sleep(Duration::from_secs_f32(0.10));

        assert!(!anim.finished());
        assert!(
            anim.value() >= 0.70 && anim.value() <= 0.74,
            "Actual: {}",
            anim.value()
        );

        sleep(Duration::from_secs_f32(0.15));

        assert!(anim.finished());
        assert!(
            anim.value() >= 0.92 && anim.value() <= 1.04,
            "Actual: {}",
            anim.value()
        );

        sleep(Duration::from_secs_f32(0.25));

        assert!(anim.finished());
        assert!(
            anim.value() >= 0.40 && anim.value() <= 0.60,
            "Actual: {}",
            anim.value()
        );
    }
}
