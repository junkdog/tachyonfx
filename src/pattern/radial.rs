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

impl Pattern for RadialPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PreparedPattern<(f32, Rect), RadialPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let pattern = &self.pattern;
        let (global_alpha, area) = self.context;

        // Calculate center position in cell coordinates
        let center_x = area.x as f32 + (pattern.center_x * area.width as f32);
        let center_y = area.y as f32 + (pattern.center_y * area.height as f32);

        // Calculate distance from center in cell coordinates
        let dx = pos.x as f32 - center_x;
        let dy = pos.y as f32 - center_y;
        // Compensate for terminal cell aspect ratio (typically 2:1 height to width)
        let distance = math::sqrt(dx * dx + 2.0 * dy * 2.0 * dy);

        // Calculate maximum radius (distance to the farthest corner) - also with aspect ratio
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
                    // Apply same aspect ratio compensation as distance calculation
                    math::sqrt(dx * dx + 2.0 * dy * 2.0 * dy)
                })
                .fold(0.0f32, f32::max)
        };

        TransitionProgress::from(pattern.transition_width).map_radial(
            global_alpha,
            distance,
            max_radius,
        )
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
            format_compact!("RadialPattern::new({}, {})", self.center_x, self.center_y)
        } else {
            format_compact!(
                "RadialPattern::with_transition(({}, {}), {})",
                self.center_x,
                self.center_y,
                self.transition_width
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use ratatui::layout::{Position, Rect};

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
            "Center should be fully active, got alpha={:.3}",
            center_alpha
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
                    "Position at {expected_distance} cells should be mostly active (inside circle radius ~2.5), got alpha={:.3}",
                    alpha
                );
            }
            // At 4-cell distance, should have lower alpha
            else if expected_distance == 4.0 {
                assert!(
                    alpha < 0.5,
                    "Position at {expected_distance} cells should have low alpha, got alpha={:.3}",
                    alpha
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
                "Alpha should be in valid range for width {width}, got alpha={:.3}",
                alpha
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
                "Center at {:.1} should have higher alpha than offset. Center: {:.3}, Offset: {:.3}",
                expected_center_x, center_alpha, offset_alpha
            );
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
            "Animation should progress: early alpha={:.3}, late alpha={:.3}",
            early_alpha,
            late_alpha
        );

        // At animation start, position should be inactive
        let start_alpha = alphas[0];
        assert!(
            start_alpha < 0.1,
            "At animation start, distant position should be inactive, got alpha={:.3}",
            start_alpha
        );

        // At animation end, position should be mostly active
        let final_alpha = alphas[10];
        assert!(
            final_alpha > 0.8,
            "At animation end, position should be mostly active, got alpha={:.3}",
            final_alpha
        );
    }
}
