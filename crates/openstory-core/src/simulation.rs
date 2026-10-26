//! Fixed-step simulation clock matching the legacy client's 8 ms update cadence.

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePlan {
    pub updates: u8,
    /// Fraction of one fixed step remaining, for render interpolation.
    pub interpolation: f32,
}

#[derive(Clone, Debug)]
pub struct FixedStepClock {
    step: Duration,
    max_catch_up_steps: u8,
    accumulator: Duration,
}

impl Default for FixedStepClock {
    fn default() -> Self {
        Self::new(Duration::from_millis(8), 5)
    }
}

impl FixedStepClock {
    pub fn new(step: Duration, max_catch_up_steps: u8) -> Self {
        assert!(!step.is_zero(), "fixed simulation step must be nonzero");
        assert!(
            max_catch_up_steps > 0,
            "at least one catch-up step is required"
        );
        Self {
            step,
            max_catch_up_steps,
            accumulator: step,
        }
    }

    /// Adds elapsed wall time, caps catch-up work, and returns the updates to run.
    pub fn advance(&mut self, elapsed: Duration) -> FramePlan {
        self.accumulator = self
            .accumulator
            .saturating_add(elapsed)
            .min(self.step.saturating_mul(u32::from(self.max_catch_up_steps)));

        let mut updates = 0;
        while self.accumulator >= self.step && updates < self.max_catch_up_steps {
            self.accumulator -= self.step;
            updates += 1;
        }

        FramePlan {
            updates,
            interpolation: (self.accumulator.as_secs_f64() / self.step.as_secs_f64()) as f32,
        }
    }

    pub fn step(&self) -> Duration {
        self.step
    }
}
