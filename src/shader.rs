use alloc::{boxed::Box, vec::Vec};
use core::fmt::Debug;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    cell_filter::FilterProcessor, cell_iter::CellIterator, pattern::AnyPattern, widget::EffectSpan,
    CellFilter, ColorSpace, Duration, EffectTimer, SimpleRng, ThreadSafetyMarker,
};

/// A trait representing a shader-like object that can be processed for a duration.
/// The `Shader` trait defines the interface for objects that can apply visual effects
/// to terminal cells over time.
///
/// When implementing this trait, you typically only need to override `execute()`. The
/// default `process()` implementation handles timer management and calls `execute()` with
/// the current alpha value. Only override `process()` if you need custom timer handling.
pub trait Shader: ThreadSafetyMarker + Debug {
    fn name(&self) -> &'static str;

    /// Processes the shader for the given duration. The default implementation:
    /// 1. Updates the timer with the given duration
    /// 2. Calls `execute()` with the current alpha value
    /// 3. Returns any overflow duration
    ///
    /// Most effects should use this default implementation and implement `execute()`
    /// instead. Only override this if you need custom timer handling.
    ///
    /// # Arguments
    /// * `duration` - The duration to process the shader for.
    /// * `buf` - A mutable reference to the `Buffer` where the shader will be applied.
    /// * `area` - The rectangular area within the buffer where the shader will be
    ///   applied.
    ///
    /// # Returns
    /// * An `Option` containing the overflow duration if the shader is done, or `None` if
    ///   it is still running.
    ///
    /// # Example
    /// ```no_compile
    /// use tachyonfx::Duration;
    /// use ratatui_core::buffer::Buffer;
    /// use ratatui_core::layout::Rect;
    ///
    /// let mut shader = MyShader::new();
    /// let area = Rect::new(0, 0, 10, 10);
    /// let mut buffer = Buffer::empty(area);
    /// let overflow = shader.process(Duration::from_millis(100), &mut buffer, area);
    /// ```
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let overflow = self.timer_mut().and_then(|t| t.process(duration));

        self.execute(duration, area, buf);

        overflow
    }

    /// Executes the shader effect after the `duration` has been applied to the timer.
    /// This is the main implementation point for most effects, and is called by the
    /// default `process()`
    ///
    /// # Arguments
    /// * `duration` - The duration to process the shader for. If a timer is associated
    ///   with the shader, it has already been updated with this duration.
    /// * `alpha` - The alpha value indicating the progress of the shader effect.
    /// * `area` - The rectangular area within the buffer where the shader will be
    ///   applied.
    /// * `buf` - A mutable reference to the `Buffer` where the shader will be applied.
    #[allow(unused_variables)]
    fn execute(&mut self, duration: Duration, area: Rect, buf: &mut Buffer) {}

    /// Creates an iterator over the cells in the specified area, filtered by the shader's
    /// cell filter.
    ///
    /// # Arguments
    /// * `buf` - A mutable reference to the `Buffer` where the shader will be applied.
    /// * `area` - The rectangular area within the buffer where the shader will be
    ///   applied.
    ///
    /// # Returns
    /// * A [CellIterator] over the cells in the specified area.
    fn cell_iter<'a>(&'a mut self, buf: &'a mut Buffer, area: Rect) -> CellIterator<'a> {
        CellIterator::new(buf, area, self.filter_processor())
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
    fn running(&self) -> bool {
        !self.done()
    }

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

    /// Sets the cell selection strategy for the shader. Has no effect on the shader
    /// if already set.
    ///
    /// # Arguments
    /// * `filter` - The cell selection strategy to set.
    ///
    /// # Example
    /// ```no_compile
    /// use ratatui_core::style::Color;
    /// use tachyonfx::{CellFilter, fx, Interpolation};
    ///
    /// let mut shader = MyShader::new();
    /// shader.set_cell_selection(CellFilter::Not(CellFilter::Text));
    /// ```
    fn filter(&mut self, filter: CellFilter);

    #[deprecated(since = "0.11.0", note = "Use `filter()` instead")]
    fn set_cell_selection(&mut self, filter: CellFilter) {
        self.filter(filter);
    }

    /// Reverses the shader effect.
    fn reverse(&mut self) {
        if let Some(timer) = self.timer_mut() {
            *timer = timer.reversed();
        }
    }

    /// Returns a mutable reference to the shader's timer, if any.
    ///
    /// # Returns
    /// * An `Option` containing a mutable reference to the shader's `EffectTimer`, or
    ///   `None` if not applicable.
    ///
    /// # Example
    /// ```no_compile
    /// let mut shader = MyShader::new();
    /// if let Some(timer) = shader.timer_mut() {
    ///     timer.reset();
    /// }
    /// ```
    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    /// Returns the timer associated with this shader effect.
    ///
    /// This method provides information about the duration and timing of the effect,
    /// useful for effect composition and synchronization.
    ///
    /// # Returns
    /// An `Option<EffectTimer>`:
    /// - `Some(EffectTimer)` if the shader has an associated timer.
    /// - `None` if the shader doesn't have a specific duration (e.g., for indefinite
    ///   effects).
    ///
    /// # Notes
    /// - For composite effects (like parallel or sequential effects), this may return an
    ///   approximation of the total duration based on the timers of child effects.
    /// - Some effects may modify the returned timer to reflect their specific behavior
    ///   (e.g., a ping-pong effect might double the duration).
    /// - The returned timer should reflect the total expected duration of the effect,
    ///   which may differ from the current remaining time.
    fn timer(&self) -> Option<EffectTimer> {
        None
    }

    /// Returns the cell selection strategy for the shader, if any.
    ///
    /// # Returns
    /// * An `Option` containing the shader's `CellFilter`, or `None` if not applicable.
    fn cell_filter(&self) -> Option<&CellFilter> {
        None
    }

    /// Returns the shader's filter processor for selective cell processing.
    ///
    /// # Returns
    /// * An `Option` containing the shader's `FilterProcessor`, or `None` if not
    ///   applicable.
    fn filter_processor(&self) -> Option<&FilterProcessor> {
        None
    }

    /// Returns a mutable reference to the shader's filter processor for selective cell
    fn filter_processor_mut(&mut self) -> Option<&mut FilterProcessor> {
        None
    }

    #[deprecated(since = "0.11.0", note = "Use `cell_filter()` instead")]
    fn cell_selection(&self) -> Option<CellFilter> {
        self.cell_filter().cloned()
    }

    /// Sets the color space used for color interpolation
    #[allow(unused_variables)]
    fn set_color_space(&mut self, color_space: ColorSpace) {}

    /// Gets the current color space. Returns the default color space (HSL) if not
    /// supported.
    fn color_space(&self) -> ColorSpace {
        ColorSpace::default()
    }

    /// Sets a pattern for spatial alpha progression. This is a no-op for effects that
    /// don't support patterns. Pattern-compatible effects should override this
    /// method.
    ///
    /// # Arguments
    /// * `pattern` - An AnyPattern enum containing the pattern to apply
    #[allow(unused_variables)]
    fn set_pattern(&mut self, pattern: AnyPattern) {
        // Default no-op implementation for non-pattern-compatible shaders
    }

    /// Sets the random number generator for stochastic effects. This is a no-op for
    /// effects that don't use randomness. Randomness-compatible effects should override
    /// this method.
    #[allow(unused_variables)]
    fn set_rng(&mut self, rng: SimpleRng) {}

    /// Resets the shader effect. Used by [fx::ping_pong](fx/fn.ping_pong.html) and
    /// [fx::repeat](fx/fn.repeat.html) to reset the hosted shader effect to its initial
    /// state.
    fn reset(&mut self) {
        if let Some(timer) = self.timer_mut() {
            timer.reset();
        } else {
            panic!("Shader must implement reset()")
        }
    }

    /// Attempts to convert this shader to a DSL effect expression.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing either:
    /// - `Ok(EffectExpression)` if conversion is successful
    /// - `Err(DslError::EffectExpressionNotSupported)` containing the shader name if this
    ///   shader type doesn't support conversion to the DSL format
    ///
    /// # Errors
    ///
    /// This default implementation always returns an error with the shader's name,
    /// indicating that DSL conversion is not supported. Shader implementations that
    /// support DSL conversion should override this method.
    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslError;
        Err(DslError::EffectExpressionNotSupported { name: self.name() })
    }

    /// Creates an `EffectSpan` representation of this shader.
    ///
    /// # Deprecation
    ///
    /// This method was used by the now-removed `EffectTimeline` widget and no longer
    /// serves any purpose. It is deprecated and scheduled for removal in a future
    /// release.
    #[deprecated(since = "0.23.0", note = "EffectSpan is being removed")]
    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, Vec::default())
    }
}

/// A macro for implementing common Shader trait functions.
///
/// This macro reduces boilerplate code by automatically implementing various
/// Shader trait methods based on the specified fields. You can choose which
/// groups of implementations to include.
///
/// # Arguments
///
/// * One or more tokens representing implementation groups:
///   - `area` - Implements `area()` and `set_area()` methods
///   - `timer` - Implements `done()`, `timer_mut()`, and `timer()` methods
///   - `filter` - Implements `filter()` and `cell_filter()` methods
///   - `color_space` - Implements `set_color_space()` and `color_space()` methods
///   - `clone` - Implements `clone_box()` method
///
/// # Requirements
///
/// Depending on which groups you include, your struct must have the following fields:
/// * `area` - Requires a field named `area` of type `Option<Rect>`
/// * `timer` - Requires a field named `timer` of type `EffectTimer`
/// * `filter` - Requires a field named `cell_filter` of type `CellFilter`
/// * `color_space` - Requires a field named `color_space` of type `ColorSpace`
/// * `clone` - Requires your type to implement `Clone`
#[macro_export]
macro_rules! default_shader_impl {
    ( $($group:ident),* ) => {
        $(
            default_shader_impl!(@$group);
        )*
    };

    // Area implementation
    (@area) => {
        fn area(&self) -> Option<Rect> {
            self.area
        }

        fn set_area(&mut self, area: Rect) {
            self.area = Some(area);
        }
    };

    // Timer implementation
    (@timer) => {
        fn done(&self) -> bool {
            self.timer.done()
        }

        fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
            Some(&mut self.timer)
        }

        fn timer(&self) -> Option<EffectTimer> {
            Some(self.timer)
        }
    };

    // Filter implementation
    (@filter) => {
        fn filter(&mut self, strategy: CellFilter) {
            self.cell_filter = Some(FilterProcessor::from(strategy));
        }

        fn cell_filter(&self) -> Option<&CellFilter> {
            self.cell_filter.as_ref().map(|f| f.filter_ref())
        }

        fn filter_processor(&self) -> Option<&FilterProcessor> {
            self.cell_filter.as_ref()
        }


        fn filter_processor_mut(&mut self) -> Option<&mut FilterProcessor> {
            self.cell_filter.as_mut()
        }
    };

    // Color space implementation
    (@color_space) => {
        fn set_color_space(&mut self, color_space: ColorSpace) {
            self.color_space = color_space;
        }

        fn color_space(&self) -> ColorSpace {
            self.color_space
        }
    };

    // Clone implementation
    (@clone) => {
        fn clone_box(&self) -> Box<dyn Shader> {
            Box::new(self.clone())
        }
    };
}
