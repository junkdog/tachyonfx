mod any_pattern;
mod checkerboard;
mod coalesce;
mod diagonal;
mod instanced_pattern;
mod radial;
mod sweep;

use ratatui::layout::Rect;

pub(crate) use self::instanced_pattern::InstancedPattern;
pub use self::{
    any_pattern::AnyPattern,
    checkerboard::CheckerboardPattern,
    coalesce::CoalescePattern,
    diagonal::{DiagonalDirection, DiagonalPattern},
    radial::RadialPattern,
    sweep::SweepPattern,
};

/// Trait for patterns that can be prepared for per-frame rendering.
///
/// Patterns define spatial effects for animations by transforming global alpha
/// values into position-specific alpha values. This trait handles the initialization
/// phase where patterns prepare their per-frame context based on the current
/// animation progress and render area.
pub(crate) trait Pattern {
    /// The context type that holds per-frame state for this pattern
    type Context;

    /// Prepares the pattern for rendering a specific frame.
    ///
    /// # Arguments
    /// * `alpha` - Global animation progress (0.0-1.0)
    /// * `area` - The rectangular area where the pattern will be applied
    ///
    /// # Returns
    /// A `PatternForFrame` instance ready for per-cell alpha computation
    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized;
}

pub(crate) struct PatternForFrame<S, P: Pattern> {
    pattern: P,
    context: S,
}
