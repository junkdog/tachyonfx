use crate::cell_iter::CellIterator;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::widget::EffectSpan;
use crate::{CellFilter, Duration};
use crate::EffectTimer;

/// A trait representing a shader-like object that can be processed for a duration.
/// The `Shader` trait defines the interface for objects that can apply visual effects
/// to terminal cells over time.
pub trait Shader {
    fn name(&self) -> &'static str;

    /// Processes the shader for the given duration. Returns any overflowed
    /// duration if the shader has completed.
    ///
    /// This default implementation calls `execute` with the alpha value and the cells.
    ///
    /// # Arguments
    /// * `duration` - The duration to process the shader for.
    /// * `buf` - A mutable reference to the `Buffer` where the shader will be applied.
    /// * `area` - The rectangular area within the buffer where the shader will be applied.
    ///
    /// # Returns
    /// * An `Option` containing the overflow duration if the shader is done, or `None`
    ///   if it is still running.
    ///
    /// # Example
    /// ```no_compile
    /// use std::time::Duration;
    /// use ratatui::buffer::Buffer;
    /// use ratatui::layout::Rect;
    ///
    /// let mut shader = MyShader::new();
    /// let area = Rect::new(0, 0, 10, 10);
    /// let mut buffer = Buffer::empty(area);
    /// let overflow = shader.process(Duration::from_millis(100), &mut buffer, area);
    /// ```
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let (overflow, alpha) = self.timer_mut()
            .map(|t| (t.process(duration), t.alpha()))
            .unwrap_or((None, 1.0));

        let requested_cells = self.cell_iter(buf, area);
        self.execute(alpha, area, requested_cells);

        overflow
    }

    /// Executes the shader with the given alpha value and cells. This is where
    /// the actual shader logic should be implemented.
    ///
    /// # Arguments
    /// * `alpha` - The alpha value indicating the progress of the shader effect.
    /// * `area` - The rectangular area within the buffer where the shader will be applied.
    /// * `cell_iter` - An iterator over the cells in the specified area.
    fn execute(
        &mut self,
        alpha: f32,
        area: Rect,
        cell_iter: CellIterator,
    );

    /// Creates an iterator over the cells in the specified area, filtered by the shader's cell filter.
    ///
    /// # Arguments
    /// * `buf` - A mutable reference to the `Buffer` where the shader will be applied.
    /// * `area` - The rectangular area within the buffer where the shader will be applied.
    ///
    /// # Returns
    /// * A `CellIterator` over the cells in the specified area.
    fn cell_iter<'a>(
        &mut self,
        buf: &'a mut Buffer,
        area: Rect,
    ) -> CellIterator<'a> {
        CellIterator::new(buf, area, self.cell_selection())
    }

    /// Returns true if the shader effect is done.
    ///
    /// # Returns
    /// * `true` if the shader effect is done, `false` otherwise.
    fn done(&self) -> bool;

    /// Returns true if the shader is still running.
    ///
    /// # Returns
    /// * `true` if the shader is running, `false` otherwise.
    fn running(&self) -> bool { !self.done() }

    /// Creates a boxed clone of the shader.
    ///
    /// # Returns
    /// * A boxed clone of the shader.
    fn clone_box(&self) -> Box<dyn Shader>;

    /// Returns the area where the shader effect is applied.
    ///
    /// # Returns
    /// * An `Option` containing the rectangular area if set, or `None` if not set.
    fn area(&self) -> Option<Rect>;

    /// Sets the area where the shader effect will be applied.
    ///
    /// # Arguments
    /// * `area` - The rectangular area to set.
    fn set_area(&mut self, area: Rect);

    /// Sets the cell selection strategy for the shader.
    ///
    /// # Arguments
    /// * `filter` - The cell selection strategy to set.
    ///
    /// # Example
    /// ```no_compile
    /// use ratatui::style::Color;
    /// use tachyonfx::{CellFilter, fx, Interpolation};
    ///
    /// let mut shader = MyShader::new();
    /// shader.set_cell_selection(CellFilter::Not(CellFilter::Text));
    /// ```
    fn set_cell_selection(&mut self, filter: CellFilter);

    /// Reverses the shader effect.
    fn reverse(&mut self) {
        if let Some(timer) = self.timer_mut() {
            *timer = timer.reversed()
        }
    }

    /// Returns a mutable reference to the shader's timer, if any.
    ///
    /// # Returns
    /// * An `Option` containing a mutable reference to the shader's `EffectTimer`, or `None` if not applicable.
    ///
    /// # Example
    /// ```no_compile
    /// let mut shader = MyShader::new();
    /// if let Some(timer) = shader.timer_mut() {
    ///     timer.reset();
    /// }
    /// ```
    fn timer_mut(&mut self) -> Option<&mut EffectTimer> { None }

    /// Returns the timer associated with this shader effect.
    ///
    /// This method is primarily used for visualization purposes, such as in the `EffectTimeline` widget.
    /// It provides information about the duration and timing of the effect.
    ///
    /// # Returns
    /// An `Option<EffectTimer>`:
    /// - `Some(EffectTimer)` if the shader has an associated timer.
    /// - `None` if the shader doesn't have a specific duration (e.g., for indefinite effects).
    ///
    /// # Notes
    /// - For composite effects (like parallel or sequential effects), this may return an approximation
    ///   of the total duration based on the timers of child effects.
    /// - Some effects may modify the returned timer to reflect their specific behavior
    ///   (e.g., a ping-pong effect might double the duration).
    /// - The returned timer should reflect the total expected duration of the effect, which may differ
    ///   from the current remaining time.
    fn timer(&self) -> Option<EffectTimer> { None }

    /// Returns the cell selection strategy for the shader, if any.
    ///
    /// # Returns
    /// * An `Option` containing the shader's `CellFilter`, or `None` if not applicable.
    fn cell_selection(&self) -> Option<CellFilter> { None }

    /// Resets the shader effect. Used by [fx::ping_pong](fx/fn.ping_pong.html) and
    /// [fx::repeat](fx/fn.repeat.html) to reset the hosted shader effect to its initial state.
    fn reset(&mut self) {
        if let Some(timer) = self.timer_mut() {
            timer.reset();
        } else {
            panic!("Shader must implement reset()")
        }
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, Vec::default())
    }
}
