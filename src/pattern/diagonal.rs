#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::{
    math,
    pattern::{InstancedPattern, Pattern, PreparedPattern, TransitionProgress},
};

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct DiagonalPattern {
    direction: DiagonalDirection,
    transition_width: f32,
}

/// Direction variants for diagonal sweep patterns.
#[derive(Clone, Debug, Copy, PartialEq)]
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
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    pub fn top_right_to_bottom_left() -> Self {
        Self {
            direction: DiagonalDirection::TopRightToBottomLeft,
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    pub fn bottom_left_to_top_right() -> Self {
        Self {
            direction: DiagonalDirection::BottomLeftToTopRight,
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    pub fn bottom_right_to_top_left() -> Self {
        Self {
            direction: DiagonalDirection::BottomRightToTopLeft,
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    /// Creates a diagonal pattern with specified direction and transition width.
    ///
    /// # Arguments
    /// * `direction` - Direction of the diagonal sweep
    /// * `transition_width` - Width of gradient transition zone in terminal cells
    ///   (minimum 0.1)
    pub fn new(direction: DiagonalDirection, transition_width: f32) -> Self {
        Self {
            direction,
            transition_width: transition_width.max(0.1),
        }
    }

    /// Sets the transition width for gradient smoothing along the diagonal edge.
    ///
    /// # Arguments
    /// * `width` - Width of gradient transition zone in terminal cells (minimum 0.1)
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.1);
        self
    }
}

impl Pattern for DiagonalPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PreparedPattern<(f32, Rect), DiagonalPattern> {
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

        // Use TransitionProgress with inverse spatial mapping for correct character evolution
        // Convert cell-based transition width to normalized units using diagonal length
        let diagonal_length =
            math::sqrt(math::powi(area.width as f32, 2) + math::powi(area.height as f32, 2));
        let normalized_transition_width = pattern.transition_width / diagonal_length;
        TransitionProgress::from(normalized_transition_width).map_spatial(
            global_alpha,
            diagonal_progress,
            1.0,
        )
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for DiagonalDirection {
    fn dsl_format(&self) -> CompactString {
        match self {
            DiagonalDirection::TopLeftToBottomRight => {
                "DiagonalDirection::TopLeftToBottomRight".to_compact_string()
            },
            DiagonalDirection::TopRightToBottomLeft => {
                "DiagonalDirection::TopRightToBottomLeft".to_compact_string()
            },
            DiagonalDirection::BottomLeftToTopRight => {
                "DiagonalDirection::BottomLeftToTopRight".to_compact_string()
            },
            DiagonalDirection::BottomRightToTopLeft => {
                "DiagonalDirection::BottomRightToTopLeft".to_compact_string()
            },
        }
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for DiagonalPattern {
    fn dsl_format(&self) -> CompactString {
        if (self.transition_width - 2.0).abs() < f32::EPSILON {
            // Use named constructor for default transition width
            match self.direction {
                DiagonalDirection::TopLeftToBottomRight => {
                    "DiagonalPattern::top_left_to_bottom_right()".to_compact_string()
                },
                DiagonalDirection::TopRightToBottomLeft => {
                    "DiagonalPattern::top_right_to_bottom_left()".to_compact_string()
                },
                DiagonalDirection::BottomLeftToTopRight => {
                    "DiagonalPattern::bottom_left_to_top_right()".to_compact_string()
                },
                DiagonalDirection::BottomRightToTopLeft => {
                    "DiagonalPattern::bottom_right_to_top_left()".to_compact_string()
                },
            }
        } else {
            // Use with_transition_width for custom transition width
            let base = match self.direction {
                DiagonalDirection::TopLeftToBottomRight => {
                    "DiagonalPattern::top_left_to_bottom_right()"
                },
                DiagonalDirection::TopRightToBottomLeft => {
                    "DiagonalPattern::top_right_to_bottom_left()"
                },
                DiagonalDirection::BottomLeftToTopRight => {
                    "DiagonalPattern::bottom_left_to_top_right()"
                },
                DiagonalDirection::BottomRightToTopLeft => {
                    "DiagonalPattern::bottom_right_to_top_left()"
                },
            };
            if self.transition_width.fract() == 0.0 {
                format_compact!(
                    "{}.with_transition_width({})",
                    base,
                    self.transition_width as u32
                )
            } else {
                format_compact!("{}.with_transition_width({})", base, self.transition_width)
            }
        }
    }
}
