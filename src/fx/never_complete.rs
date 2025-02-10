use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{CellFilter, Duration, EffectTimer};
use crate::dsl::{DslError, EffectExpression};
use crate::effect::Effect;
use crate::widget::EffectSpan;
use crate::shader::Shader;

#[derive(Clone, Debug)]
pub struct NeverComplete {
    effect: Effect,
}

impl NeverComplete {
    pub fn new(effect: Effect) -> Self {
        Self { effect }
    }
}

impl Shader for NeverComplete {
    fn name(&self) -> &'static str {
        "never_complete"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.effect.process(duration, buf, area);
        None
    }

    fn done(&self) -> bool                      { false }
    fn clone_box(&self) -> Box<dyn Shader>      { Box::new(self.clone()) }
    fn area(&self) -> Option<Rect>              { self.effect.area() }
    fn set_area(&mut self, area: Rect)          { self.effect.set_area(area) }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.effect.set_cell_selection(strategy);
    }

    fn reverse(&mut self) {
        self.effect.reverse()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.effect.cell_selection()
    }

    fn reset(&mut self) {
        self.effect.reset();
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.effect.as_effect_span(offset)])
    }

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        let nested = self.effect.to_dsl()?;
        EffectExpression::parse(&format!("never_complete({})", nested))
    }
}

#[cfg(test)]
mod tests {
    use crate::fx::{consume_tick, never_complete};
    use crate::Shader;

    #[test]
    fn to_dsl() {
        let dsl = never_complete(consume_tick())
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, "fx::never_complete(fx::consume_tick())");
    }
}