use ratatui::layout::{Position, Rect};

use crate::pattern::{InstancedPattern, Pattern, PatternForFrame};

#[derive(Clone, Debug, Copy)]
pub struct DiagonalPattern {
    direction: DiagonalDirection,
    transition_width: f32,
}

/// Direction variants for diagonal sweep patterns.
#[derive(Clone, Debug, Copy)]
pub enum DiagonalDirection {
    /// Sweeps diagonally from top-left corner to bottom-right corner
    TopLeftToBottomRight,
    /// Sweeps diagonally from top-right corner to bottom-left corner
    TopRightToBottomLeft,
    /// Sweeps diagonally from bottom-left corner to top-right corner
    BottomLeftToTopRight,
    /// Sweeps diagonally from bottom-right corner to top-left corner
    BottomRightToTopLeft,
}

impl DiagonalPattern {
    pub fn top_left_to_bottom_right() -> Self {
        Self {
            direction: DiagonalDirection::TopLeftToBottomRight,
            transition_width: 0.1,
        }
    }

    pub fn top_right_to_bottom_left() -> Self {
        Self {
            direction: DiagonalDirection::TopRightToBottomLeft,
            transition_width: 0.1,
        }
    }

    pub fn bottom_left_to_top_right() -> Self {
        Self {
            direction: DiagonalDirection::BottomLeftToTopRight,
            transition_width: 0.1,
        }
    }

    pub fn bottom_right_to_top_left() -> Self {
        Self {
            direction: DiagonalDirection::BottomRightToTopLeft,
            transition_width: 0.1,
        }
    }

    /// Creates a diagonal pattern with specified direction and transition width.
    ///
    /// # Arguments
    /// * `direction` - Direction of the diagonal sweep
    /// * `transition_width` - Width of gradient transition zone (0.01-1.0, automatically
    ///   clamped)
    pub fn new(direction: DiagonalDirection, transition_width: f32) -> Self {
        Self {
            direction,
            transition_width: transition_width.clamp(0.01, 1.0),
        }
    }

    /// Sets the transition width for gradient smoothing along the diagonal edge.
    ///
    /// # Arguments
    /// * `width` - Width of gradient transition zone (0.01-1.0, automatically clamped)
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.clamp(0.01, 1.0);
        self
    }
}

impl Pattern for DiagonalPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized,
    {
        PatternForFrame { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PatternForFrame<(f32, Rect), DiagonalPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let pattern = &self.pattern;
        let (global_alpha, area) = self.context;

        // Normalize position to 0.0-1.0 range
        let norm_x = (pos.x - area.x) as f32 / area.width as f32;
        let norm_y = (pos.y - area.y) as f32 / area.height as f32;

        // Calculate diagonal progress based on direction
        use DiagonalDirection::*;
        let diagonal_progress = match pattern.direction {
            TopLeftToBottomRight => (norm_x + norm_y) / 2.0,
            TopRightToBottomLeft => ((1.0 - norm_x) + norm_y) / 2.0,
            BottomLeftToTopRight => (norm_x + (1.0 - norm_y)) / 2.0,
            BottomRightToTopLeft => ((1.0 - norm_x) + (1.0 - norm_y)) / 2.0,
        };

        let transition_width = pattern.transition_width;

        // Scale global_alpha to include transition zone
        let scaled_alpha = global_alpha * (1.0 + transition_width) - (transition_width / 2.0);

        if diagonal_progress <= scaled_alpha {
            // Fully active
            1.0
        } else if diagonal_progress <= scaled_alpha + transition_width {
            // Transition zone
            let distance_into_transition = diagonal_progress - scaled_alpha;
            let progress = 1.0 - (distance_into_transition / transition_width);
            progress.clamp(0.0, 1.0)
        } else {
            // Inactive
            0.0
        }
    }
}
