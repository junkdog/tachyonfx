use ratatui::layout::Rect;
use crate::Duration;

use crate::CellFilter;
use crate::dsl::{DslError, EffectExpression};
use crate::effect_timer::EffectTimer;
use crate::widget::EffectSpan;
use crate::shader::Shader;

#[derive(Clone, Debug)]
pub struct Sleep {
    timer: EffectTimer,
}

impl Sleep {
    pub fn new<T: Into<EffectTimer>>(duration: T) -> Self {
        Self { timer: duration.into() }
    }
}

impl Shader for Sleep {
    fn name(&self) -> &'static str {
        "sleep"
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn set_cell_selection(&mut self, _strategy: CellFilter) {}

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, Vec::default())
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.timer.reset();
    }

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        EffectExpression::parse(&format!("fx::sleep({})", self.timer.duration().as_millis()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{fx, Shader};

    #[test]
    fn to_dsl() {
        let dsl = fx::sleep(1000).to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::sleep(1000)");
    }
}

