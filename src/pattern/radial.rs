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
pub struct RadialPattern {
    center_x: f32,
    center_y: f32,
    transition_width: f32,
}

impl RadialPattern {
    /// Creates a radial pattern centered at the middle of the area with default
    /// transition width
    pub fn center() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    /// Creates a radial pattern with custom center point (0.0-1.0 normalized coordinates)
    /// and default transition width
    pub fn new(center_x: f32, center_y: f32) -> Self {
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: 2.0, // Default to 2 terminal cells
        }
    }

    /// Creates a radial pattern with custom center and transition width
    ///
    /// # Arguments
    /// * `center_x` - Center X position (0.0-1.0 normalized coordinates)
    /// * `center_y` - Center Y position (0.0-1.0 normalized coordinates)
    /// * `transition_width` - Width of the gradient transition zone in terminal cells
    ///   (minimum 0.1)
    pub fn with_transition(center: (f32, f32), transition_width: f32) -> Self {
        let (center_x, center_y) = center;
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: transition_width.max(0.1),
        }
    }

    /// Sets the transition width for gradient smoothing
    ///
    /// # Arguments
    /// * `width` - Width of the gradient transition zone in terminal cells (minimum 0.1).
    ///   Smaller values create sharper edges.
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.1);
        self
    }

    /// Sets a custom center point for the radial pattern
    pub fn with_center(mut self, center: (f32, f32)) -> Self {
        let (center_x, center_y) = center;
        self.center_x = center_x.clamp(0.0, 1.0);
        self.center_y = center_y.clamp(0.0, 1.0);
        self
    }
}

/// Precomputed per-frame state for [`RadialPattern`].
pub struct RadialContext {
    center_x: f32,
    center_y: f32,
    /// Distance threshold below which cells are fully active.
    threshold: f32,
    /// Distance threshold above which cells are fully inactive.
    threshold_end: f32,
    /// Reciprocal of transition width for fast linear interpolation.
    inv_transition_width: f32,
}

impl Pattern for RadialPattern {
    type Context = RadialContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let center_x = area.x as f32 + (self.center_x * area.width as f32);
        let center_y = area.y as f32 + (self.center_y * area.height as f32);
        let transition_width = self.transition_width.max(0.1);

        let max_radius = {
            let corners = [
                (area.x as f32, area.y as f32),
                (area.right() as f32, area.y as f32),
                (area.x as f32, area.bottom() as f32),
                (area.right() as f32, area.bottom() as f32),
            ];
            corners
                .iter()
                .map(|(x, y)| {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    math::sqrt(dx * dx + 4.0 * dy * dy)
                })
                .fold(0.0f32, f32::max)
        };

        let threshold = (alpha * (max_radius + 2.0 * transition_width)) - transition_width;

        PreparedPattern {
            pattern: self,
            context: RadialContext {
                center_x,
                center_y,
                threshold,
                threshold_end: threshold + transition_width,
                inv_transition_width: 1.0 / transition_width,
            },
        }
    }
}

impl InstancedPattern for PreparedPattern<RadialContext, RadialPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let ctx = &self.context;

        let dx = pos.x as f32 - ctx.center_x;
        let dy = pos.y as f32 - ctx.center_y;
        // Compensate for terminal cell aspect ratio (typically 2:1 height to width)
        let distance = math::sqrt(dx * dx + 4.0 * dy * dy);

        if distance <= ctx.threshold {
            1.0
        } else if distance <= ctx.threshold_end {
            let distance_into_transition = distance - ctx.threshold;
            1.0 - (distance_into_transition * ctx.inv_transition_width).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for RadialPattern {
    fn dsl_format(&self) -> CompactString {
        if (self.center_x - 0.5).abs() < f32::EPSILON
            && (self.center_y - 0.5).abs() < f32::EPSILON
            && (self.transition_width - 2.0).abs() < f32::EPSILON
        {
            "RadialPattern::center()".to_compact_string()
        } else if (self.transition_width - 2.0).abs() < f32::EPSILON {
            format_compact!(
                "RadialPattern::new({}, {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y)
            )
        } else {
            format_compact!(
                "RadialPattern::with_transition(({}, {}), {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y),
                fmt_f32(self.transition_width)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use ratatui_core::layout::{Position, Rect};

    use super::*;

    #[test]
    fn test_radial_transition_width_coverage() {
        // Use 10x1 area for simple horizontal distance calculations
        let area = Rect::new(0, 0, 10, 1);
        let pattern = RadialPattern::center().with_transition_width(2.0); // 2 terminal cells

        // Test at 50% animation progress
        let mut prepared = pattern.for_frame(0.5, area);

        // Center is at (5, 0) in this 10x1 area
        let center_pos = Position::new(5, 0);
        let center_alpha = prepared.map_alpha(center_pos);

        // Center should be fully active
        assert!(
            center_alpha > 0.9,
            "Center should be fully active, got alpha={center_alpha:.3}"
        );

        // Test positions at known horizontal distances from center
        let test_cases = [
            (Position::new(3, 0), 2.0), // 2 cells left of center
            (Position::new(7, 0), 2.0), // 2 cells right of center
            (Position::new(1, 0), 4.0), // 4 cells left of center
            (Position::new(9, 0), 4.0), // 4 cells right of center
        ];

        for (pos, expected_distance) in test_cases {
            let alpha = prepared.map_alpha(pos);

            // At 2-cell distance, circle radius is 2.5, so this should be fully active
            if expected_distance == 2.0 {
                assert!(
                    alpha > 0.8,
                    "Position at {expected_distance} cells should be mostly active (inside circle radius ~2.5), got alpha={alpha:.3}"
                );
            }
            // At 4-cell distance, should have lower alpha
            else if expected_distance == 4.0 {
                assert!(
                    alpha < 0.5,
                    "Position at {expected_distance} cells should have low alpha, got alpha={alpha:.3}"
                );
            }
        }
    }

    #[test]
    fn test_radial_transition_width_scaling() {
        // Use 10x1 area for simple calculations
        let area = Rect::new(0, 0, 10, 1);

        // Test different transition widths
        let transition_widths = [1.0, 2.0, 4.0];

        for &width in &transition_widths {
            let pattern = RadialPattern::center().with_transition_width(width);
            let mut prepared = pattern.for_frame(0.5, area);

            // Test position 2 cells from center
            let test_pos = Position::new(7, 0); // 2 cells right of center
            let alpha = prepared.map_alpha(test_pos);

            // At 50% progress, most positions will be inside the expanded circle
            // Just verify that larger transition widths don't break the logic
            assert!(
                (0.0..=1.0).contains(&alpha),
                "Alpha should be in valid range for width {width}, got alpha={alpha:.3}"
            );

            // For small width, edges should be sharper (but position might still be
            // inside circle) For large width, gradients should be smoother
            // The key is that the function produces valid results
        }
    }

    #[test]
    fn test_radial_different_centers() {
        // Use 10x1 area for simple calculations
        let area = Rect::new(0, 0, 10, 1);
        let transition_width = 2.0;

        // Test different center positions
        let centers = [
            (0.0, 0.0), // Left edge
            (1.0, 0.0), // Right edge
            (0.5, 0.0), // Center
            (0.2, 0.0), // Off-center left
        ];

        for &(center_x, _) in &centers {
            let pattern = RadialPattern::new(center_x, 0.0).with_transition_width(transition_width);
            let mut prepared = pattern.for_frame(0.4, area);

            // Calculate expected center position
            let expected_center_x = center_x * area.width as f32;

            // Test that the actual center position has highest alpha
            let center_pos = Position::new(expected_center_x as u16, 0);
            let center_alpha = prepared.map_alpha(center_pos);

            // Test a position 2 cells away from center (if within bounds)
            let offset_x = if expected_center_x >= 2.0 {
                expected_center_x - 2.0
            } else {
                expected_center_x + 2.0
            };
            let offset_pos = Position::new(offset_x as u16, 0);
            let offset_alpha = prepared.map_alpha(offset_pos);

            // Center should have higher or equal alpha than offset position
            assert!(
                center_alpha >= offset_alpha,
                "Center at {expected_center_x:.1} should have higher alpha than offset. Center: {center_alpha:.3}, Offset: {offset_alpha:.3}"
            );
        }
    }

    #[test]
    fn test_radial_boundary_alphas() {
        // 10x1 area, center at (5,0), transition_width=2.0
        // At alpha=0.0:  threshold = 0*(max_r+4)-2 = -2, so all cells inactive (0.0)
        // At alpha=1.0:  threshold = 1*(max_r+4)-2 = max_r+2, so all cells active (1.0)
        let area = Rect::new(0, 0, 10, 1);
        let pattern = RadialPattern::center().with_transition_width(2.0);

        // alpha=0 => all cells should be 0.0
        let mut p = pattern.for_frame(0.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 0.0, "alpha=0: expected 0.0 at x={x}, got {a}");
        }

        // alpha=1 => all cells should be 1.0
        let mut p = pattern.for_frame(1.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 1.0, "alpha=1: expected 1.0 at x={x}, got {a}");
        }
    }

    #[test]
    fn test_radial_symmetry() {
        // Cells equidistant from center should have equal alpha
        let area = Rect::new(0, 0, 20, 1);
        let pattern = RadialPattern::center().with_transition_width(2.0);
        let mut p = pattern.for_frame(0.5, area);

        // center at x=10; check x=8 vs x=12 (both 2 cells away)
        let left = p.map_alpha(Position::new(8, 0));
        let right = p.map_alpha(Position::new(12, 0));
        assert!(
            (left - right).abs() < 1e-5,
            "Symmetric positions should have equal alpha: left={left}, right={right}"
        );
    }

    #[test]
    fn test_radial_monotonic_from_center() {
        // Alpha should be monotonically non-increasing as distance from center grows
        let area = Rect::new(0, 0, 40, 1);
        let pattern = RadialPattern::center().with_transition_width(3.0);
        let mut p = pattern.for_frame(0.5, area);

        let center_x = 20u16;
        let mut prev_alpha = p.map_alpha(Position::new(center_x, 0));
        for d in 1..20u16 {
            let a = p.map_alpha(Position::new(center_x + d, 0));
            assert!(
                a <= prev_alpha + 1e-5,
                "Alpha should decrease with distance: at d={d}, got {a} > prev {prev_alpha}"
            );
            prev_alpha = a;
        }
    }

    #[test]
    fn test_radial_animation_progression() {
        // Use 10x1 area for simple calculations
        let area = Rect::new(0, 0, 10, 1);
        let pattern = RadialPattern::center().with_transition_width(2.0);

        // Test position 3 cells from center
        let test_pos = Position::new(8, 0); // 3 cells right of center (5)

        let mut alphas = Vec::new();

        // Sample alpha values at different animation stages
        for i in 0..=10 {
            let global_alpha = i as f32 / 10.0; // 0.0 to 1.0
            let mut prepared = pattern.for_frame(global_alpha, area);
            let alpha = prepared.map_alpha(test_pos);
            alphas.push(alpha);
        }

        // Verify progression: alpha should generally increase over time
        let early_alpha = alphas[2]; // At 20% progress
        let late_alpha = alphas[8]; // At 80% progress

        assert!(
            late_alpha > early_alpha,
            "Animation should progress: early alpha={early_alpha:.3}, late alpha={late_alpha:.3}"
        );

        // At animation start, position should be inactive
        let start_alpha = alphas[0];
        assert!(
            start_alpha < 0.1,
            "At animation start, distant position should be inactive, got alpha={start_alpha:.3}"
        );

        // At animation end, position should be mostly active
        let final_alpha = alphas[10];
        assert!(
            final_alpha > 0.8,
            "At animation end, position should be mostly active, got alpha={final_alpha:.3}"
        );
    }
}
