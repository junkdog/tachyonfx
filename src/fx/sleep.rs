use alloc::boxed::Box;

use ratatui_core::layout::Rect;

use crate::{default_shader_impl, effect_timer::EffectTimer, shader::Shader, CellFilter};

#[derive(Clone, Debug)]
pub(super) struct Sleep {
    timer: EffectTimer,
}

impl Sleep {
    pub fn new<T: Into<EffectTimer>>(duration: T) -> Self {
        Self { timer: duration.into() }
    }
}

impl Shader for Sleep {
    default_shader_impl!(timer, clone);

    fn name(&self) -> &'static str {
        "sleep"
    }

    fn area(&self) -> Option<Rect> {
        None
    }
    fn set_area(&mut self, _area: Rect) {}
    fn filter(&mut self, _strategy: CellFilter) {}

    fn cell_filter(&self) -> Option<&CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.timer.reset();
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse(&format!(
            "fx::sleep({})",
            self.timer.duration().as_millis()
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use crate::fx;

    #[test]
    fn to_dsl() {
        let dsl = fx::sleep(1000).to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::sleep(1000)");
    }
}
