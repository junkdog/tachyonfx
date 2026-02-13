#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui_core::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::{dsl_format::fmt_f32, DslFormat};
use crate::{
    math,
    pattern::{InstancedPattern, Pattern, PreparedPattern},
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

/// Precomputed per-frame state for [`DiagonalPattern`].
pub struct DiagonalContext {
    /// `global_alpha * (1.0 + 2.0 * tw) - tw`
    threshold: f32,
    /// `threshold + transition_width`
    threshold_end: f32,
    /// `1.0 / transition_width`
    inv_transition_width: f32,
    inv_width: f32,
    inv_height: f32,
    area_x: f32,
    area_y: f32,
    direction: DiagonalDirection,
}

impl Pattern for DiagonalPattern {
    type Context = DiagonalContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let diagonal_length =
            math::sqrt(math::powi(area.width as f32, 2) + math::powi(area.height as f32, 2));
        let normalized_tw = (self.transition_width / diagonal_length).max(0.1);

        // Precompute the scaled alpha from TransitionProgress::map_spatial
        // with max_range=1.0:
        //   threshold = alpha * (1.0 + 2.0 * tw) - tw
        let threshold = alpha * (1.0 + 2.0 * normalized_tw) - normalized_tw;

        PreparedPattern {
            pattern: self,
            context: DiagonalContext {
                threshold,
                threshold_end: threshold + normalized_tw,
                inv_transition_width: 1.0 / normalized_tw,
                inv_width: 1.0 / area.width as f32,
                inv_height: 1.0 / area.height as f32,
                area_x: area.x as f32,
                area_y: area.y as f32,
                direction: self.direction,
            },
        }
    }
}

impl InstancedPattern for PreparedPattern<DiagonalContext, DiagonalPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let ctx = &self.context;

        // Normalize position to 0.0-1.0 range (multiply by precomputed reciprocal)
        let norm_x = (pos.x as f32 - ctx.area_x) * ctx.inv_width;
        let norm_y = (pos.y as f32 - ctx.area_y) * ctx.inv_height;

        // Calculate diagonal progress based on direction
        use DiagonalDirection::*;
        let diagonal_progress = match ctx.direction {
            TopLeftToBottomRight => (norm_x + norm_y) * 0.5,
            TopRightToBottomLeft => ((1.0 - norm_x) + norm_y) * 0.5,
            BottomLeftToTopRight => (norm_x + (1.0 - norm_y)) * 0.5,
            BottomRightToTopLeft => ((1.0 - norm_x) + (1.0 - norm_y)) * 0.5,
        };

        if diagonal_progress <= ctx.threshold {
            1.0
        } else if diagonal_progress <= ctx.threshold_end {
            let distance_into_transition = diagonal_progress - ctx.threshold;
            (1.0 - (distance_into_transition * ctx.inv_transition_width)).clamp(0.0, 1.0)
        } else {
            0.0
        }
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
            format_compact!(
                "{}.with_transition_width({})",
                base,
                fmt_f32(self.transition_width)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::layout::{Position, Rect};

    use super::*;

    #[test]
    fn test_diagonal_boundary_alphas() {
        let area = Rect::new(0, 0, 10, 10);
        let pattern = DiagonalPattern::top_left_to_bottom_right();

        // alpha=0 => all cells should be 0.0
        let mut p = pattern.for_frame(0.0, area);
        for y in 0..10 {
            for x in 0..10 {
                let a = p.map_alpha(Position::new(x, y));
                assert!(a == 0.0, "alpha=0: expected 0.0 at ({x},{y}), got {a}");
            }
        }

        // alpha=1 => all cells should be 1.0
        let mut p = pattern.for_frame(1.0, area);
        for y in 0..10 {
            for x in 0..10 {
                let a = p.map_alpha(Position::new(x, y));
                assert!(a == 1.0, "alpha=1: expected 1.0 at ({x},{y}), got {a}");
            }
        }
    }

    #[test]
    fn test_diagonal_tl_br_ordering() {
        // For TL→BR, top-left corner (0,0) has progress 0.0, bottom-right (9,9) has ~1.0
        // At mid-animation, top-left should be more active
        let area = Rect::new(0, 0, 10, 10);
        let pattern = DiagonalPattern::top_left_to_bottom_right();
        let mut p = pattern.for_frame(0.5, area);

        let tl = p.map_alpha(Position::new(0, 0));
        let br = p.map_alpha(Position::new(9, 9));
        assert!(
            tl > br,
            "TL→BR at alpha=0.5: top-left ({tl}) should be more active than bottom-right ({br})"
        );
    }

    #[test]
    fn test_diagonal_br_tl_ordering() {
        // For BR→TL, bottom-right corner activates first
        let area = Rect::new(0, 0, 10, 10);
        let pattern = DiagonalPattern::bottom_right_to_top_left();
        let mut p = pattern.for_frame(0.5, area);

        let tl = p.map_alpha(Position::new(0, 0));
        let br = p.map_alpha(Position::new(9, 9));
        assert!(
            br > tl,
            "BR→TL at alpha=0.5: bottom-right ({br}) should be more active than top-left ({tl})"
        );
    }

    #[test]
    fn test_diagonal_monotonic_progression() {
        // Along the diagonal axis, alpha should be monotonically non-increasing
        let area = Rect::new(0, 0, 20, 20);
        let pattern = DiagonalPattern::top_left_to_bottom_right();
        let mut p = pattern.for_frame(0.5, area);

        let mut prev = p.map_alpha(Position::new(0, 0));
        for d in 1..20u16 {
            let a = p.map_alpha(Position::new(d, d));
            assert!(
                a <= prev + 1e-5,
                "Alpha should not increase along diagonal: at d={d}, got {a} > prev {prev}"
            );
            prev = a;
        }
    }

    #[test]
    fn test_diagonal_equi_progress_symmetry() {
        // Cells with the same diagonal progress should have the same alpha.
        // For TL→BR, progress = (norm_x + norm_y) / 2.
        // (2,0) and (0,2) have the same progress in a square area.
        let area = Rect::new(0, 0, 10, 10);
        let pattern = DiagonalPattern::top_left_to_bottom_right();
        let mut p = pattern.for_frame(0.5, area);

        let a1 = p.map_alpha(Position::new(2, 0));
        let a2 = p.map_alpha(Position::new(0, 2));
        assert!(
            (a1 - a2).abs() < 1e-5,
            "Cells on same anti-diagonal should match: ({a1}) vs ({a2})"
        );
    }

    #[test]
    fn test_diagonal_animation_increases_coverage() {
        let area = Rect::new(0, 0, 10, 10);
        let pattern = DiagonalPattern::top_left_to_bottom_right();
        let pos = Position::new(5, 5); // mid-area

        let mut prev = 0.0f32;
        for i in 0..=10 {
            let alpha = i as f32 / 10.0;
            let mut p = pattern.for_frame(alpha, area);
            let a = p.map_alpha(pos);
            assert!(
                a >= prev - 1e-5,
                "Alpha at pos (5,5) should increase over time: alpha={alpha}, got {a} < prev {prev}"
            );
            prev = a;
        }
    }
}
