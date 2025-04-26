use crate::widget::EffectSpan;
use crate::shader::Shader;
use crate::{CellFilter, ColorSpace, Duration, EffectTimer};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Represents an effect that can be applied to terminal cells.
/// The `Effect` struct wraps a shader, allowing it to be configured
/// and applied to a specified area and cell selection.
#[derive(Debug)]
pub struct Effect {
    shader: Box<dyn Shader>,
}

impl Effect {
    /// Creates a new `Effect` with the specified shader.
    ///
    /// # Arguments
    /// * `shader` - The shader to be used for the effect. It must implement the `Shader` trait and have a static lifetime.
    ///
    /// # Returns
    /// * A new `Effect` instance.
    pub fn new<S>(shader: S) -> Self
        where S: Shader + 'static
    {
        Self { shader: Box::new(shader) }
    }

    /// Creates a new `Effect` with the specified area.
    ///
    /// # Arguments
    /// * `area` - The rectangular area where the effect will be applied.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified area.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{Effect, EffectTimer, fx, Interpolation};
    /// use ratatui::layout::Rect;
    ///
    /// fx::dissolve(EffectTimer::from_ms(120, Interpolation::CircInOut))
    ///     .with_area(Rect::new(0, 0, 10, 10));
    /// ```
    pub fn with_area(mut self, area: Rect) -> Self {
        self.shader.set_area(area);
        self
    }

    /// Creates a new `Effect` with the specified cell filter.
    ///
    /// # Arguments
    /// * `mode` - The terminal cell filter to be used for the effect.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified filter.
    ///
    /// /// # Notes
    /// This method only applies the filter if the effect doesn't already have a filter set,
    /// preserving any existing filters during effect composition.
    /// 
    /// # Example
    /// ```
    /// use ratatui::style::Color;
    /// use tachyonfx::{Effect, CellFilter, fx, Interpolation};
    /// use tachyonfx::color_from_hsl;
    ///
    /// let color = color_from_hsl(180.0, 85.0, 62.0);
    /// let shader = fx::fade_to_fg(color, (300, Interpolation::SineIn))
    ///     .with_filter(CellFilter::Text);
    /// ```
    pub fn with_filter(mut self, mode: CellFilter) -> Self {
        self.filter(mode);
        self
    }

    #[deprecated(since = "0.11.0", note = "Use `with_filter` instead")]
    pub fn with_cell_selection(&self, mode: CellFilter) -> Self {
        self.clone().with_filter(mode)
    }


    pub fn color_space(&self) -> ColorSpace {
        self.shader.color_space()
    }

    pub fn set_color_space(&mut self, color_space: ColorSpace) {
        self.shader.set_color_space(color_space);
    }

    pub fn with_color_space(mut self, color_space: ColorSpace) -> Self {
        self.set_color_space(color_space);
        self
    }

    /// Creates a new `Effect` with the shader's reverse flag toggled.
    ///
    /// # Returns
    /// * A new `Effect` instance with the shader's reverse flag toggled.
    pub fn reversed(&self) -> Self {
        let mut cloned = self.clone();
        cloned.reverse();
        cloned
    }
}


impl Clone for Effect {
    fn clone(&self) -> Self {
        Self { shader: self.shader.clone_box() }
    }
}

impl Shader for Effect {
    fn name(&self) -> &'static str {
        self.shader.name()
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let area = self.shader.area().unwrap_or(area);
        self.shader.process(duration, buf, area)
    }

    fn execute(&mut self, duration: Duration, area: Rect, buf: &mut Buffer) {
        self.shader.execute(duration, area, buf);
    }

    fn done(&self) -> bool {
        self.shader.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        self.shader.clone_box()
    }

    fn area(&self) -> Option<Rect> {
        self.shader.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.shader.set_area(area)
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.shader.propagate_filter(strategy)
    }

    fn reverse(&mut self) {
        self.shader.reverse()
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.shader.timer()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        self.shader.timer_mut()
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        self.shader.cell_filter()
    }

    fn reset(&mut self) {
        self.shader.reset()
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan
    where
        Self: Sized + Clone,
    {
        self.shader.as_ref().as_effect_span(offset)
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        self.shader.to_dsl()
    }
}


pub trait IntoEffect {
    fn into_effect(self) -> Effect;
}

impl<S> IntoEffect for S
    where S: Shader + 'static
{
    fn into_effect(self) -> Effect {
        Effect::new(self)
    }
}


pub(crate) trait ShaderExt {
    /// Propagates the cell filter to the shader if it is not already set.
    fn propagate_filter(&mut self, cell_filter: CellFilter);
}

impl <S: Shader + 'static> ShaderExt for S {
    fn propagate_filter(&mut self, cell_filter: CellFilter) {
        if self.cell_filter().is_none() {
            self.filter(cell_filter);
        }
    }
}

impl ShaderExt for dyn Shader {
    fn propagate_filter(&mut self, cell_filter: CellFilter) {
        if self.cell_filter().is_none() {
            self.filter(cell_filter);
        }
    }
}