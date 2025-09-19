mod any_pattern;
mod checkerboard;
mod coalesce;
mod diagonal;
mod dissolve;
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
    dissolve::DissolvePattern,
    radial::RadialPattern,
    sweep::SweepPattern,
};

/// Trait for patterns that can be prepared for per-frame rendering.
///
/// Patterns define spatial effects for animations by transforming global alpha
/// values into position-specific alpha values. This trait handles the initialization
/// phase where patterns prepare their per-frame context based on the current
/// animation progress and render area.
pub trait Pattern {
    /// The context type that holds per-frame state for this pattern
    type Context;

    /// Prepares the pattern for rendering a specific frame.
    ///
    /// # Arguments
    /// * `alpha` - Global animation progress (0.0-1.0)
    /// * `area` - The rectangular area where the pattern will be applied
    ///
    /// # Returns
    /// A `PreparedPattern` instance ready for per-cell alpha computation
    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized;
}

/// A pattern that has been prepared for rendering a specific frame.
///
/// This struct wraps a pattern along with its frame-specific context, created by
/// calling [`Pattern::for_frame`]. It contains all the necessary state to compute
/// per-cell alpha values for the current animation frame.
///
/// # Type Parameters
/// * `S` - The context type that holds per-frame state for the pattern
/// * `P` - The pattern type that implements [`Pattern`]
pub struct PreparedPattern<S, P: Pattern> {
    pattern: P,
    context: S,
}

/// Encapsulates alpha progression with smooth transitions.
///
/// This type handles the common pattern of mapping global animation progress to
/// position-specific alpha values with smooth gradient transitions. It provides
/// consistent scaling formulas and parameter ordering across different pattern types.
#[derive(Debug, Clone, Copy)]
pub struct TransitionProgress {
    transition_width: f32,
}

impl From<f32> for TransitionProgress {
    /// Creates a new transition progress handler with the specified transition width.
    ///
    /// # Arguments
    /// * `width` - Width of the gradient transition zone in terminal cells (minimum 0.1)
    fn from(width: f32) -> Self {
        Self {
            transition_width: width.max(0.1), // Allow any positive value >= 0.1 cells
        }
    }
}

impl TransitionProgress {
    /// Maps spatial patterns where positions vary along a continuous dimension.
    ///
    /// Used for patterns like diagonal and radial where positions have a natural
    /// progression along some spatial dimension with a defined range.
    ///
    /// # Arguments
    /// * `global_alpha` - Global animation progress (0.0-1.0)
    /// * `position` - Position along the pattern dimension
    /// * `max_range` - Maximum value for the position dimension
    pub fn map_spatial(&self, global_alpha: f32, position: f32, max_range: f32) -> f32 {
        // Scale global_alpha to include transition zone
        let scaled_alpha =
            global_alpha * (max_range + 2.0 * self.transition_width) - self.transition_width;

        if position <= scaled_alpha {
            // Fully active (positions before threshold)
            1.0
        } else if position <= scaled_alpha + self.transition_width {
            // Transition zone with correct falloff
            let distance_into_transition = position - scaled_alpha;
            let progress = 1.0 - (distance_into_transition / self.transition_width);
            progress.clamp(0.0, 1.0)
        } else {
            // Inactive
            0.0
        }
    }

    /// Maps radial patterns where smaller distances (closer to center) should be more
    /// active.
    ///
    /// # Arguments
    /// * `global_alpha` - Global animation progress (0.0-1.0)
    /// * `distance` - Distance from center point
    /// * `max_range` - Maximum distance value
    pub fn map_radial(&self, global_alpha: f32, distance: f32, max_range: f32) -> f32 {
        let threshold =
            (global_alpha * (max_range + 2.0 * self.transition_width)) - self.transition_width;

        // For radial: we want 1.0 when distance <= threshold, 0.0 when distance > threshold +
        // transition
        if distance <= threshold {
            1.0
        } else if distance <= threshold + self.transition_width {
            // In transition zone - linear falloff
            let distance_into_transition = distance - threshold;
            1.0 - (distance_into_transition / self.transition_width).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Maps discrete threshold patterns where cells have distinct activation thresholds.
    ///
    /// Used for patterns like checkerboard where cells belong to discrete categories
    /// with specific threshold values.
    ///
    /// # Arguments
    /// * `global_alpha` - Global animation progress (0.0-1.0)
    /// * `cell_threshold` - The discrete threshold for this cell (e.g., 0.0 for white,
    ///   0.5 for black)
    pub fn map_threshold(&self, global_alpha: f32, cell_threshold: f32) -> f32 {
        let transition_width = self.transition_width;

        // Scale the alpha range to include transition zones
        let scaled_alpha = global_alpha * (1.0 + transition_width) - (transition_width / 2.0);

        if scaled_alpha >= cell_threshold + transition_width {
            // Fully active
            1.0
        } else if scaled_alpha >= cell_threshold {
            // Transition zone
            let progress_into_transition = scaled_alpha - cell_threshold;
            let progress = progress_into_transition / transition_width;
            progress.clamp(0.0, 1.0)
        } else {
            // Inactive
            0.0
        }
    }
}
