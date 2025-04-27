use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use Interpolation::Linear;
use crate::{default_shader_impl, CellFilter, ColorSpace, Duration, Effect, EffectTimer, Interpolation, Shader};
use crate::widget::EffectSpan;

#[derive(Debug, Clone)]
pub struct FreezeAt {
    alpha: f32,
    set_raw_alpha: bool, 
    fx: Effect
}

/// An effect that freezes another effect at a specific alpha (transition) value.
///
/// `FixedAlpha` sets the inner effect to a specific alpha value and keeps it there
/// indefinitely, effectively "freezing" the effect at that stage of its transition.
///
/// # Examples
///
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use ratatui::style::Color;
///
/// // Create a fade effect that stops at 70% of its transition
/// let fade = fx::fade_to_fg(Color::Red, (1000, Interpolation::Linear));
/// let frozen_fade = fx::freeze_at(0.7, false, fade);
/// ```
impl FreezeAt {
    pub fn new(
        alpha: f32,
        set_raw_alpha: bool,
        fx: Effect
    ) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        Self { alpha, fx, set_raw_alpha }
    }
}

impl Shader for FreezeAt {
    default_shader_impl!(clone);
    
    fn name(&self) -> &'static str {
        "freeze_at"
    }

    fn process(&mut self, _duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        if let Some(t) = self.fx.timer_mut() {
            // fix alpha on first frame
            if !t.started() {
                let interpolation = if self.set_raw_alpha { Linear } else { t.interpolation() };
                let d = t.remaining() * (1.0 - self.alpha);
                *t = EffectTimer::new(t.remaining(), interpolation);
                t.process(d);
            }
        }
        
        self.fx.process(Duration::from_millis(0), buf, area)
    }

    fn done(&self) -> bool {
        false
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area)
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.fx.timer()
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        self.fx.cell_filter()
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse(&format!(
            "fx::freeze_at({}, {}, {})",
            self.alpha,
            self.set_raw_alpha,
            self.fx.to_dsl()?
        ))
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.fx.as_effect_span(offset)])
    }

    fn reset(&mut self) {
        self.fx.reset();
    }
}