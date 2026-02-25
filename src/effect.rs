use alloc::boxed::Box;
use core::fmt::Debug;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    pattern::AnyPattern, shader::Shader, widget::EffectSpan, CellFilter, ColorSpace, Duration,
    EffectTimer, SimpleRng,
};

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
    /// * `shader` - The shader to be used for the effect. It must implement the `Shader`
    ///   trait and have a static lifetime.
    ///
    /// # Returns
    /// * A new `Effect` instance.
    pub fn new<S>(shader: S) -> Self
    where
        S: Shader + 'static,
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
    /// use ratatui_core::layout::Rect;
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
    /// * `mode` - The cell filter to be used for the effect.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified filter.
    ///
    /// # Notes
    /// This method only applies the filter if the effect doesn't already have a filter
    /// set, preserving any existing filters during effect composition.
    ///
    /// # Example
    /// ```
    /// use ratatui_core::style::Color;
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

    /// Gets the current color space used for color interpolation.
    /// Returns the default color space (HSL) if not supported.
    ///
    /// # Returns
    /// * The `ColorSpace` currently in use by this effect.
    pub fn color_space(&self) -> ColorSpace {
        self.shader.color_space()
    }

    /// Sets the color space used for color interpolation.
    ///
    /// # Arguments
    /// * `color_space` - The color space to use for color interpolation.
    pub fn set_color_space(&mut self, color_space: ColorSpace) {
        self.shader.set_color_space(color_space);
    }

    /// Sets the color space for color interpolation in the effect.
    ///
    /// This method controls how colors are blended during transitions, which can
    /// significantly affect the visual appearance of the animation. Different color
    /// spaces produce different intermediate colors during transitions.
    ///
    /// # Color Spaces
    ///
    /// - [`ColorSpace::Rgb`](crate::ColorSpace::Rgb) - Linear RGB interpolation (fastest,
    ///   but can produce muddy colors)
    /// - [`ColorSpace::Hsl`](crate::ColorSpace::Hsl) - HSL interpolation (default, smooth
    ///   hue transitions)
    /// - [`ColorSpace::Hsv`](crate::ColorSpace::Hsv) - HSV interpolation (vibrant,
    ///   perceptually uniform)
    ///
    /// # Supported Effects
    ///
    /// The following effects support `with_color_space()`:
    /// - [`fx::fade_to`](crate::fx::fade_to), [`fx::fade_from`](crate::fx::fade_from) -
    ///   Controls foreground and background color transitions
    /// - [`fx::fade_to_fg`](crate::fx::fade_to_fg),
    ///   [`fx::fade_from_fg`](crate::fx::fade_from_fg) - Controls foreground color
    ///   transitions
    /// - [`fx::sweep_in`](crate::fx::sweep_in), [`fx::sweep_out`](crate::fx::sweep_out) -
    ///   Controls color fading during sweep
    ///
    /// For effects that don't use color interpolation, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ratatui_core::style::Color;
    /// use tachyonfx::{ColorSpace, fx, Interpolation};
    ///
    /// let effect = fx::fade_to_fg(Color::Red, (300, Interpolation::SineIn))
    ///     .with_color_space(ColorSpace::Rgb);
    /// ```
    pub fn with_color_space(mut self, color_space: ColorSpace) -> Self {
        self.set_color_space(color_space);
        self
    }

    /// Sets the random number generator for the effect, enabling reproducible animations.
    ///
    /// This method allows you to control the randomness in effects that use random
    /// number generation, making it possible to create deterministic, reproducible
    /// animations by seeding the RNG with a fixed value.
    ///
    /// # Supported Effects
    ///
    /// The following effects support `with_rng()`:
    /// - [`fx::glitch`](crate::fx::glitch) - Controls random cell selection and glitch
    ///   types
    /// - [`fx::dissolve`](crate::fx::dissolve),
    ///   [`fx::dissolve_to`](crate::fx::dissolve_to),
    ///   [`fx::coalesce`](crate::fx::coalesce),
    ///   [`fx::coalesce_from`](crate::fx::coalesce_from) - Controls random cell
    ///   thresholds
    /// - [`fx::explode`](crate::fx::explode) - Controls explosion forces and trajectories
    /// - [`fx::slide_in`](crate::fx::slide_in), [`fx::slide_out`](crate::fx::slide_out) -
    ///   Controls random positional variance
    /// - [`fx::sweep_in`](crate::fx::sweep_in), [`fx::sweep_out`](crate::fx::sweep_out) -
    ///   Controls random positional variance
    ///
    /// For effects that don't use randomness, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::{fx, SimpleRng, Effect};
    ///
    /// // Create two effects with the same seed for reproducible behavior
    /// let effect1 = fx::dissolve(1000).with_rng(SimpleRng::new(42));
    /// let effect2 = fx::dissolve(1000).with_rng(SimpleRng::new(42));
    /// // effect1 and effect2 will dissolve cells in the exact same pattern
    /// ```
    pub fn with_rng(mut self, rng: SimpleRng) -> Self {
        self.shader.set_rng(rng);
        self
    }

    /// Creates a new `Effect` with the shader's reverse flag toggled.
    ///
    /// # Returns
    /// * A new `Effect` instance with the shader's reverse flag toggled.
    pub fn reversed(mut self) -> Self {
        self.reverse();
        self
    }
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self { shader: self.shader.clone_box() }
    }
}

impl Effect {
    /// Returns the name of the underlying shader.
    ///
    /// # Returns
    /// * The name of the shader as a static string.
    pub fn name(&self) -> &'static str {
        self.shader.name()
    }

    /// Processes the effect for the given duration. This:
    /// 1. Updates the shader's timer with the given duration
    /// 2. Executes the shader effect
    /// 3. Returns any overflow duration
    ///
    /// # Arguments
    /// * `duration` - The duration to process the effect for.
    /// * `buf` - A mutable reference to the `Buffer` where the effect will be applied.
    /// * `area` - The rectangular area within the buffer where the effect will be
    ///   applied. If the effect has its own area set, that takes precedence.
    ///
    /// # Returns
    /// * An `Option` containing the overflow duration if the effect is done, or `None` if
    ///   it is still running.
    ///
    /// # Example
    /// ```no_run
    /// use ratatui_core::buffer::Buffer;
    /// use ratatui_core::layout::Rect;
    /// use tachyonfx::{Effect, fx, Interpolation, Duration};
    ///
    /// let mut effect = fx::dissolve((100, Interpolation::Linear));
    /// let area = Rect::new(0, 0, 10, 10);
    /// let mut buffer = Buffer::empty(area);
    /// let overflow = effect.process(Duration::from_millis(50), &mut buffer, area);
    /// ```
    pub fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let area = self.shader.area().unwrap_or(area);
        if let Some(processor) = self.shader.filter_processor_mut() {
            processor.update(buf, area);
        }

        self.shader.process(duration, buf, area)
    }

    /// Returns true if the effect is done.
    ///
    /// # Returns
    /// * `true` if the effect is done, `false` otherwise.
    pub fn done(&self) -> bool {
        self.shader.done()
    }

    /// Returns true if the effect is still running.
    ///
    /// # Returns
    /// * `true` if the effect is running, `false` otherwise.
    pub fn running(&self) -> bool {
        self.shader.running()
    }

    /// Returns the area where the effect is applied.
    ///
    /// # Returns
    /// * An `Option` containing the rectangular area if set, or `None` if not set.
    pub fn area(&self) -> Option<Rect> {
        self.shader.area()
    }

    /// Sets the area where the effect will be applied.
    ///
    /// # Arguments
    /// * `area` - The rectangular area to set.
    pub fn set_area(&mut self, area: Rect) {
        self.shader.set_area(area);
    }

    /// Sets the cell selection strategy for the effect. Only applies the filter
    /// if the effect doesn't already have one set.
    ///
    /// # Arguments
    /// * `strategy` - The cell filter strategy to set.
    ///
    /// # Example
    /// ```no_run
    /// use tachyonfx::{CellFilter, fx, Interpolation};
    ///
    /// let mut effect = fx::dissolve((100, Interpolation::Linear));
    /// effect.filter(CellFilter::Not(CellFilter::Text.into()));
    /// ```
    pub fn filter(&mut self, strategy: CellFilter) {
        self.shader.propagate_filter(strategy);
    }

    /// Reverses the effect's playback direction.
    ///
    /// This toggles the effect to play in the opposite direction from its
    /// current state. Can be used to create back-and-forth animations.
    pub fn reverse(&mut self) {
        self.shader.reverse();
    }

    /// Returns the timer associated with this effect.
    ///
    /// This method provides information about the duration and timing of the effect,
    /// useful for effect composition and synchronization.
    ///
    /// # Returns
    /// An `Option<EffectTimer>`:
    /// - `Some(EffectTimer)` if the effect has an associated timer.
    /// - `None` if the effect doesn't have a specific duration (e.g., for indefinite
    ///   effects).
    ///
    /// # Notes
    /// - For composite effects (like parallel or sequential effects), this may return an
    ///   approximation of the total duration based on the timers of child effects.
    /// - Some effects may modify the returned timer to reflect their specific behavior
    ///   (e.g., a ping-pong effect might double the duration).
    /// - The returned timer should reflect the total expected duration of the effect,
    ///   which may differ from the current remaining time.
    pub fn timer(&self) -> Option<EffectTimer> {
        self.shader.timer()
    }

    /// Returns a mutable reference to the effect's timer, if any.
    ///
    /// # Returns
    /// * An `Option` containing a mutable reference to the effect's `EffectTimer`, or
    ///   `None` if not applicable.
    ///
    /// # Example
    /// ```no_run
    /// use tachyonfx::{fx, Interpolation};
    ///
    /// let mut effect = fx::dissolve((100, Interpolation::Linear));
    /// if let Some(timer) = effect.timer_mut() {
    ///     timer.reset();
    /// }
    /// ```
    pub fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        self.shader.timer_mut()
    }

    /// Returns the cell selection strategy for the effect, if any.
    ///
    /// # Returns
    /// * An `Option` containing the effect's `CellFilter`, or `None` if not applicable.
    pub fn cell_filter(&self) -> Option<&CellFilter> {
        self.shader.cell_filter()
    }

    /// Resets the effect. Used by effects like ping_pong and repeat to reset
    /// the hosted effect to its initial state.
    pub fn reset(&mut self) {
        self.shader.reset();
    }

    /// Sets a pattern for spatial alpha progression on pattern-compatible effects.
    /// This is a no-op for effects that don't support patterns.
    ///
    /// # Arguments
    /// * `pattern` - An AnyPattern enum containing the pattern to apply
    pub(crate) fn set_pattern(&mut self, pattern: AnyPattern) {
        self.shader.set_pattern(pattern);
    }

    /// Applies a pattern to this effect for spatial alpha progression.
    /// This is a no-op for effects that don't support patterns.
    ///
    /// # Arguments
    /// * `pattern` - A pattern that implements `Into<AnyPattern>`
    ///
    /// # Returns
    /// * The same effect with the pattern applied (if supported)
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{fx, pattern};
    ///
    /// let effect = fx::dissolve(1000)
    ///     .with_pattern(pattern::RadialPattern::center());
    /// ```
    pub fn with_pattern<P>(mut self, pattern: P) -> Self
    where
        P: Into<AnyPattern>,
    {
        let any_pattern = pattern.into();
        self.set_pattern(any_pattern);
        self
    }

    /// Creates an `EffectSpan` representation of this effect.
    ///
    /// # Deprecation
    ///
    /// This method was used by the now-removed `EffectTimeline` widget and no longer
    /// serves any purpose. It is deprecated and scheduled for removal in a future
    /// release.
    #[deprecated(since = "0.23.0", note = "EffectSpan is being removed")]
    #[allow(deprecated)]
    pub fn as_effect_span(&self, offset: Duration) -> EffectSpan
    where
        Self: Sized + Clone,
    {
        self.shader.as_ref().as_effect_span(offset)
    }

    /// Attempts to convert this effect to a DSL effect expression.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing either:
    /// - `Ok(EffectExpression)` if conversion is successful
    /// - `Err(DslError::EffectExpressionNotSupported)` containing the effect name if this
    ///   effect type doesn't support conversion to the DSL format
    ///
    /// # Errors
    ///
    /// This method returns an error if the underlying shader doesn't support DSL
    /// conversion.
    #[cfg(feature = "dsl")]
    pub fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        self.shader.to_dsl()
    }
}

/// Trait for converting shader types into Effect instances.
pub trait IntoEffect {
    /// Converts this shader into an Effect.
    fn into_effect(self) -> Effect;
}

impl<S> IntoEffect for S
where
    S: Shader + 'static,
{
    fn into_effect(self) -> Effect {
        Effect::new(self)
    }
}

/// Extension trait for shader filter propagation logic.
pub(crate) trait ShaderExt {
    /// Propagates the cell filter to the shader if it is not already set.
    ///
    /// This method only applies the filter if the shader doesn't already have one,
    /// preserving existing filters during effect composition.
    fn propagate_filter(&mut self, cell_filter: CellFilter);
}

impl<S: Shader + 'static> ShaderExt for S {
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

// PatternedEffect is no longer needed since Effect now has .with_pattern() directly
