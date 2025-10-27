#[cfg(feature = "dsl")]
use compact_str::{CompactString, ToCompactString};
use ratatui::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::{
    pattern::{InstancedPattern, Pattern, PreparedPattern},
    SimpleRng,
};

/// A pattern that creates organic, randomized reveal effects.
///
/// The coalesce pattern assigns each cell a random threshold value, causing cells to
/// activate at different points during the animation. This creates a scattered,
/// organic-looking transition that resembles particles or pixels coalescing together.
///
/// Unlike structured patterns (checkerboard, radial), coalesce creates truly random
/// distributions that feel natural and unpredictable.
#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct CoalescePattern {
    rng: SimpleRng,
}

impl CoalescePattern {
    /// Creates a new coalesce pattern with a default random number generator.
    ///
    /// The coalesce pattern creates a randomized reveal effect where each cell
    /// becomes active at a random threshold, creating a scattered, organic-looking
    /// transition as the global alpha increases.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pattern for CoalescePattern {
    type Context = (f32, SimpleRng);

    fn for_frame(self, alpha: f32, _area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern { pattern: self, context: (alpha, self.rng) }
    }
}

impl InstancedPattern for PreparedPattern<(f32, SimpleRng), CoalescePattern> {
    fn map_alpha(&mut self, _pos: Position) -> f32 {
        // initial RNG is reset each frame to ensure consistent randomness
        let threshold = self.context.1.gen_f32();
        let global_alpha = self.context.0;

        // Create a smooth transition based on how far global_alpha exceeds the threshold
        // This provides a gradient effect while still maintaining randomness per cell
        if global_alpha <= threshold {
            0.0
        } else {
            // Smooth transition from 0.0 to 1.0 based on how much we exceed the threshold
            let progress = (global_alpha - threshold) / (1.0 - threshold);
            progress.clamp(0.0, 1.0)
        }
    }
}

impl From<SimpleRng> for CoalescePattern {
    fn from(rng: SimpleRng) -> Self {
        Self { rng }
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for CoalescePattern {
    fn dsl_format(&self) -> CompactString {
        "CoalescePattern::new()".to_compact_string()
    }
}
