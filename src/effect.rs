use std::time::Duration;

use crate::widget::EffectSpan;
use crate::shader::Shader;
use crate::{CellFilter, CellIterator, EffectTimer};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Represents an effect that can be applied to terminal cells.
/// The `Effect` struct wraps a shader, allowing it to be configured
/// and applied to a specified area and cell selection.
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
    pub fn with_area(&self, area: Rect) -> Self {
        let mut cloned = self.clone();
        cloned.shader.set_area(area);
        cloned
    }

    /// Creates a new `Effect` with the specified cell selection mode.
    ///
    /// # Arguments
    /// * `mode` - The cell selection mode to be used for the effect.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified cell selection mode.
    ///
    /// # Example
    /// ```
    /// use ratatui::style::Color;
    /// use tachyonfx::{Effect, CellFilter, fx, Interpolation};
    ///
    /// let color = Color::from_hsl(180.0, 85.0, 62.0);
    /// let shader = fx::fade_to_fg(color, (300, Interpolation::SineIn))
    ///     .with_cell_selection(CellFilter::Text);
    /// ```
    pub fn with_cell_selection(&self, mode: CellFilter) -> Self {
        let mut cloned = self.clone();
        cloned.set_cell_selection(mode);
        cloned
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

    fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator){
        self.shader.execute(alpha, area, cell_iter);
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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.shader.set_cell_selection(strategy)
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

    fn cell_selection(&self) -> Option<CellFilter> {
        self.shader.cell_selection()
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
