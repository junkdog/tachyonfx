use std::time::Duration;
use crate::interpolation::Interpolation;

#[derive(Clone, Default)]
pub struct EffectTimer {
    remaining: Duration,
    total: Duration,
    interpolation: Interpolation,
    reverse: bool
}

impl EffectTimer {
    pub fn from_ms(
        duration: u32,
        interpolation: Interpolation,
    ) -> Self {
        Self::new(Duration::from_millis(duration as u64), interpolation)
    }

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
    
    pub fn reversed(self) -> Self {
        Self { reverse: !self.reverse, ..self }
    }

    pub fn started(&self) -> bool {
        self.total != self.remaining
    }

    pub fn alpha(&self) -> f32 {
        let total = self.total.as_secs_f32();
        if total == 0.0 {
            return 1.0;
        }

        let remaining = self.remaining.as_secs_f32();
        let inv_alpha = remaining / total; //.clamp(0.0, 1.0);

        let a = if self.reverse { inv_alpha } else { 1.0 - inv_alpha };
        self.interpolation.alpha(a)
    }

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