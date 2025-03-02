use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::Duration;
use crate::CellFilter;
use crate::shader::Shader;

/// consumes any remaining duration for a single tick.
#[derive(Default, Clone, Debug)]
pub struct ConsumeTick {
    has_consumed_tick: bool,
}

impl Shader for ConsumeTick {
    fn name(&self) -> &'static str {
        "consume_tick"
    }

    fn process(
        &mut self,
        _duration: Duration,
        _buf: &mut Buffer,
        _area: Rect,
    ) -> Option<Duration> {
        self.has_consumed_tick = true;
        None
    }

    fn done(&self) -> bool { self.has_consumed_tick }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn filter(&mut self, _strategy: CellFilter) {}

    fn reset(&mut self) {
        self.has_consumed_tick = false;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse("fx::consume_tick()")
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use crate::{fx, Shader};

    #[test]
    fn consume_tick() {
        let dsl = fx::consume_tick().to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::consume_tick()");
    }
}