#[cfg(feature = "dsl")]
use compact_str::{CompactString, ToCompactString};
use ratatui::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::{
    pattern::{InstancedPattern, Pattern, PreparedPattern},
    SimpleRng,
};

/// A pattern that creates organic, randomized dissolve effects.
///
/// The dissolve pattern assigns each cell a random threshold value, causing cells to
/// deactivate at different points during the animation. This creates a scattered,
/// organic-looking transition that resembles particles or pixels dissolving away.
///
/// Unlike structured patterns (checkerboard, radial), dissolve creates truly random
/// distributions that feel natural and unpredictable. It's the reverse of coalesce -
/// where coalesce reveals cells as alpha increases, dissolve hides them.
#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct DissolvePattern {
    rng: SimpleRng,
}

impl DissolvePattern {
    /// Creates a new dissolve pattern with a default random number generator.
    ///
    /// The dissolve pattern creates a randomized hide effect where each cell
    /// becomes inactive at a random threshold, creating a scattered, organic-looking
    /// transition as the global alpha increases.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pattern for DissolvePattern {
    type Context = (f32, SimpleRng);

    fn for_frame(self, alpha: f32, _area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern { pattern: self, context: (alpha, self.rng) }
    }
}

impl InstancedPattern for PreparedPattern<(f32, SimpleRng), DissolvePattern> {
    fn map_alpha(&mut self, _pos: Position) -> f32 {
        // initial RNG is reset each frame to ensure consistent randomness
        let threshold = self.context.1.gen_f32();
        let global_alpha = self.context.0;

        // Dissolve is the reverse of coalesce - cells start active and dissolve away
        // Create a smooth transition based on how far global_alpha exceeds the threshold
        // This provides a gradient effect while still maintaining randomness per cell
        if global_alpha >= threshold {
            0.0 // Cell has dissolved
        } else {
            // Smooth transition from 1.0 to 0.0 based on how close we are to the threshold
            let progress = (threshold - global_alpha) / threshold;
            progress.clamp(0.0, 1.0)
        }
    }
}

impl From<SimpleRng> for DissolvePattern {
    fn from(rng: SimpleRng) -> Self {
        Self { rng }
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for DissolvePattern {
    fn dsl_format(&self) -> CompactString {
        "DissolvePattern::new()".to_compact_string()
    }
}
