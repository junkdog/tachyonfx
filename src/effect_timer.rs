use std::time::Duration;
use crate::interpolation::Interpolation;

/// A struct for managing the timing and interpolation of effects.
/// The `EffectTimer` controls the duration and progress of an effect, allowing it to be reversed,
/// reset, and processed over time.
///
/// # Fields
/// * `remaining` - The remaining duration of the effect.
/// * `total` - The total duration of the effect.
/// * `interpolation` - The interpolation method used for the effect.
/// * `reverse` - A flag indicating whether the effect is reversed.
#[derive(Clone, Copy, Default)]
pub struct EffectTimer {
    remaining: Duration,
    total: Duration,
    interpolation: Interpolation,
    reverse: bool
}

impl EffectTimer {

    /// Creates a new `EffectTimer` with the specified duration in milliseconds and interpolation method.
    ///
    /// # Arguments
    /// * `duration` - The duration of the effect in milliseconds.
    /// * `interpolation` - The interpolation method to be used for the effect.
    ///
    /// # Returns
    /// * A new `EffectTimer` instance.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// ```
    pub fn from_ms(
        duration: u32,
        interpolation: Interpolation,
    ) -> Self {
        Self::new(Duration::from_millis(duration as u64), interpolation)
    }

    /// Creates a new `EffectTimer` with the specified duration and interpolation method.
    ///
    /// # Arguments
    /// * `duration` - The duration of the effect as a `Duration` object.
    /// * `interpolation` - The interpolation method to be used for the effect.
    ///
    /// # Returns
    /// * A new `EffectTimer` instance.
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::new(Duration::from_secs(1), Interpolation::Linear);
    /// ```
    pub fn new(
        duration: Duration,
        interpolation: Interpolation,
    ) -> Self {
        Self {
            remaining: duration,
            total: duration,
            interpolation,
            reverse: false
        }
    }

    /// Reverses the timer direction.
    ///
    /// # Returns
    /// * A new `EffectTimer` instance with the reverse flag toggled.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::from_ms(1000, Interpolation::Linear).reversed();
    /// ```
    pub fn reversed(self) -> Self {
        Self { reverse: !self.reverse, ..self }
    }

    /// Checks if the timer has started.
    ///
    /// # Returns
    /// * `true` if the timer has started, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// assert!(!timer.started());
    /// ```
    pub fn started(&self) -> bool {
        self.total != self.remaining
    }

    /// Resets the timer to its initial duration.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let mut timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// timer.reset();
    /// ```
    pub fn reset(&mut self) {
        self.remaining = self.total;
    }

    /// Computes the current alpha value based on the elapsed time and interpolation method.
    ///
    /// # Returns
    /// * The current alpha value as a `f32`.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// let alpha = timer.alpha();
    /// ```
    pub fn alpha(&self) -> f32 {
        let total = self.total.as_secs_f32();
        if total == 0.0 {
            return 1.0;
        }

        let remaining = self.remaining.as_secs_f32();
        let inv_alpha = remaining / total;

        let a = if self.reverse { inv_alpha } else { 1.0 - inv_alpha };
        self.interpolation.alpha(a)
    }

    /// Processes the timer by reducing the remaining duration by the specified amount.
    ///
    /// # Arguments
    /// * `duration` - The amount of time to process.
    ///
    /// # Returns
    /// * An `Option` containing the overflow duration if the timer has completed, or `None` if the timer is still running.
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let mut timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// let overflow = timer.process(Duration::from_millis(500));
    /// assert!(overflow.is_none());
    /// ```
    pub fn process(&mut self, duration: Duration) -> Option<Duration> {
        if self.remaining >= duration {
            self.remaining -= duration;
            None
        } else {
            let overflow = duration - self.remaining;
            self.remaining = Duration::ZERO;
            Some(overflow)
        }
    }

    /// Checks if the timer has completed.
    ///
    /// # Returns
    /// * `true` if the timer has completed, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{EffectTimer, Interpolation};
    /// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
    /// assert!(!timer.done());
    /// ```
    pub fn done(&self) -> bool {
        self.remaining.is_zero()
    }
}

impl From<u32> for EffectTimer {
    fn from(ms: u32) -> Self {
        EffectTimer::new(Duration::from_millis(ms as u64), Interpolation::Linear)
    }
}

impl From<(u32, Interpolation)> for EffectTimer {
    fn from((ms, algo): (u32, Interpolation)) -> Self {
        EffectTimer::new(Duration::from_millis(ms as u64), algo)
    }
}

impl From<(Duration, Interpolation)> for EffectTimer {
    fn from((duration, algo): (Duration, Interpolation)) -> Self {
        EffectTimer::new(duration, algo)
    }
}

impl From<Duration> for EffectTimer {
    fn from(duration: Duration) -> Self {
        EffectTimer::new(duration, Interpolation::Linear)
    }
}