use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{CellFilter, Duration};
use crate::dsl::{DslError, DslFormat, EffectExpression};
use crate::effect::{Effect, IntoEffect};
use crate::effect_timer::EffectTimer;
use crate::widget::EffectSpan;
use crate::interpolation::Interpolation::Linear;
use crate::shader::Shader;

#[derive(Clone, Debug)]
pub struct TemporaryEffect {
    effect: Effect,
    timer: EffectTimer,
}

impl TemporaryEffect {
    pub fn new(effect: Effect, duration: Duration) -> Self {
        Self { effect, timer: EffectTimer::new(duration, Linear) }
    }
}

impl Shader for TemporaryEffect {
    fn name(&self) -> &'static str {
        "with_duration"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let remaining = self.timer.process(duration);
        let effect_area = self.effect.area().unwrap_or(area);
        self.effect.process(duration, buf, effect_area);
        remaining
    }

    fn done(&self) -> bool {
        self.timer.done() || self.effect.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.effect.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.effect.set_area(area)
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.effect.set_cell_selection(strategy);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.effect.as_effect_span(offset)])
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.effect.cell_selection()
    }

    fn reset(&mut self) {
        self.effect.reset();
        self.timer.reset();
    }

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        EffectExpression::parse(&format!("fx::with_duration({}, {})",
            self.timer.duration().dsl_format(),
            self.effect.to_dsl()?)
        )
    }
}

pub trait IntoTemporaryEffect {
    fn with_duration(self, duration: Duration) -> Effect;
}

impl IntoTemporaryEffect for Effect {
    fn with_duration(self, duration: Duration) -> Effect {
        TemporaryEffect::new(self, duration).into_effect()
    }
}


#[cfg(test)]
mod tests {
    use indoc::indoc;
    use crate::duration::duration::Duration;
    use crate::{fx, Shader};

    #[test]
    fn to_dsl() {
        let dsl = fx::with_duration(Duration::from_millis(1000), fx::sleep(100))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl.to_string(), indoc! {
            "fx::with_duration(
                Duration::from_millis(1000),
                fx::sleep(100)
            )"
        });
    }
}