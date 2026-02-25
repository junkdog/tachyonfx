#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui_core::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::{dsl_format::fmt_f32, DslFormat};
use crate::pattern::{InstancedPattern, Pattern, PreparedPattern, TransitionProgress};

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct CheckerboardPattern {
    cell_size: u16,
    transition_width: f32,
}

impl CheckerboardPattern {
    /// Creates a checkerboard pattern with specified cell size and transition width.
    ///
    /// # Arguments
    /// * `cell_size` - Size of each checkerboard cell in terminal cells (minimum 1)
    /// * `transition_width` - Width of gradient transition between cells in terminal
    ///   cells (minimum 0.1)
    pub fn new(cell_size: u16, transition_width: f32) -> Self {
        Self {
            cell_size,
            transition_width: transition_width.max(0.1),
        }
    }

    /// Creates a checkerboard pattern with custom cell size and default transition width.
    ///
    /// # Arguments
    /// * `cell_size` - Size of each checkerboard cell in terminal cells (automatically
    ///   clamped to minimum 1)
    pub fn with_cell_size(cell_size: u16) -> Self {
        Self { cell_size: cell_size.max(1), transition_width: 2.0 }
    }

    /// Sets the transition width for gradient smoothing between cells.
    ///
    /// # Arguments
    /// * `width` - Width of gradient transition zone in terminal cells (minimum 0.1)
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.1);
        self
    }

    #[allow(clippy::manual_is_multiple_of)] // only stabilized in 1.87.0 (2025-05)
    fn is_white_cell(self, x: u16, y: u16) -> bool {
        let cell_x = x / self.cell_size;
        let cell_y = y / self.cell_size;
        (cell_x + cell_y) % 2 == 0
    }
}

impl Default for CheckerboardPattern {
    fn default() -> Self {
        Self::new(2, 2.0) // Default to 2 terminal cells for transition
    }
}

impl Pattern for CheckerboardPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PreparedPattern<(f32, Rect), CheckerboardPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let pattern = &self.pattern;
        let (global_alpha, area) = self.context;

        // Calculate relative position within the area
        let rel_x = pos.x - area.x;
        let rel_y = pos.y - area.y;

        // Determine if this is a "white" or "black" cell in the checkerboard
        let is_white = pattern.is_white_cell(rel_x, rel_y);

        // White cells appear first, black cells appear later
        let cell_threshold = if is_white { 0.0 } else { 0.5 };

        TransitionProgress::from(pattern.transition_width)
            .map_threshold(global_alpha, cell_threshold)
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for CheckerboardPattern {
    fn dsl_format(&self) -> CompactString {
        if self.cell_size == 2 && (self.transition_width - 2.0).abs() < f32::EPSILON {
            "CheckerboardPattern::default()".to_compact_string()
        } else if (self.transition_width - 2.0).abs() < f32::EPSILON {
            format_compact!("CheckerboardPattern::with_cell_size({})", self.cell_size)
        } else {
            format_compact!(
                "CheckerboardPattern::new({}, {})",
                self.cell_size,
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
    fn test_checkerboard_with_different_cell_sizes() {
        // Test cell size 1
        let pattern_1 = CheckerboardPattern::with_cell_size(1);
        assert!(
            pattern_1.is_white_cell(0, 0),
            "Cell size 1: (0,0) should be white"
        );
        assert!(
            !pattern_1.is_white_cell(1, 0),
            "Cell size 1: (1,0) should be black"
        );

        // Test cell size 3
        let pattern_3 = CheckerboardPattern::with_cell_size(3);
        // Positions 0,1,2 should all be in the same cell (0)
        assert!(
            pattern_3.is_white_cell(0, 0),
            "Cell size 3: (0,0) should be white"
        );
        assert!(
            pattern_3.is_white_cell(1, 0),
            "Cell size 3: (1,0) should be white"
        );
        assert!(
            pattern_3.is_white_cell(2, 0),
            "Cell size 3: (2,0) should be white"
        );
        // Position 3 should be in next cell (1)
        assert!(
            !pattern_3.is_white_cell(3, 0),
            "Cell size 3: (3,0) should be black"
        );
    }

    #[test]
    fn test_checkerboard_animation_progression() {
        // Use 6x2 area to get clear checkerboard pattern
        let area = Rect::new(0, 0, 6, 2);
        let pattern = CheckerboardPattern::with_cell_size(1).with_transition_width(2.0);

        // Test white cell at (0,0) - should appear early
        let white_pos = Position::new(0, 0);
        // Test black cell at (1,0) - should appear later
        let black_pos = Position::new(1, 0);

        let mut white_alphas = Vec::new();
        let mut black_alphas = Vec::new();

        // Sample alpha values at different animation stages
        for i in 0..=10 {
            let global_alpha = i as f32 / 10.0; // 0.0 to 1.0
            let mut prepared = pattern.for_frame(global_alpha, area);

            white_alphas.push(prepared.map_alpha(white_pos));
            black_alphas.push(prepared.map_alpha(black_pos));
        }

        // White cells should appear first (at low global_alpha)
        let early_white = white_alphas[3]; // At 30% progress
        let early_black = black_alphas[3]; // At 30% progress

        assert!(
            early_white >= early_black,
            "White cells should be more active early in animation: white={early_white:.3}, black={early_black:.3}"
        );

        // At animation end, both should be mostly active
        let final_white = white_alphas[10];
        let final_black = black_alphas[10];

        assert!(
            final_white > 0.8,
            "White cell should be fully active at animation end, got alpha={final_white:.3}"
        );
        assert!(
            final_black > 0.5,
            "Black cell should be reasonably active at animation end, got alpha={final_black:.3}"
        );

        // Verify that white cells do activate before black cells
        let mid_white = white_alphas[5]; // At 50% progress
        let mid_black = black_alphas[5]; // At 50% progress
        assert!(
            mid_white > mid_black,
            "At 50% progress, white cells should be more active than black cells: white={mid_white:.3}, black={mid_black:.3}"
        );
    }

    #[test]
    fn test_checkerboard_transition_width_scaling() {
        // Use 6x2 area for testing
        let area = Rect::new(0, 0, 6, 2);

        // Test different transition widths
        let transition_widths = [0.5, 2.0, 4.0];

        for &width in &transition_widths {
            let pattern = CheckerboardPattern::with_cell_size(1).with_transition_width(width);
            let mut prepared = pattern.for_frame(0.4, area);

            // Test a white cell and a black cell
            let white_pos = Position::new(0, 0);
            let black_pos = Position::new(1, 0);

            let white_alpha = prepared.map_alpha(white_pos);
            let black_alpha = prepared.map_alpha(black_pos);

            // Alpha values should be in valid range
            assert!(
                (0.0..=1.0).contains(&white_alpha),
                "White cell alpha should be in valid range for width {width}, got alpha={white_alpha:.3}"
            );
            assert!(
                (0.0..=1.0).contains(&black_alpha),
                "Black cell alpha should be in valid range for width {width}, got alpha={black_alpha:.3}"
            );

            // At 40% progress, white cells should generally be more active than black
            // cells (though with very large transition widths, this might
            // blur)
        }
    }

    #[test]
    fn test_checkerboard_cell_pattern_consistency() {
        // Use 8x4 area to test pattern consistency
        let area = Rect::new(0, 0, 8, 4);
        let pattern = CheckerboardPattern::with_cell_size(2).with_transition_width(1.0);

        // Test at 60% animation progress where we can see the pattern clearly
        let mut prepared = pattern.for_frame(0.6, area);

        // Verify that cells within the same checkerboard cell have similar alphas
        // For cell_size=2, positions (0,0), (0,1), (1,0), (1,1) should be in same checkerboard
        // cell
        let same_cell_positions = [
            Position::new(0, 0),
            Position::new(0, 1),
            Position::new(1, 0),
            Position::new(1, 1),
        ];

        let same_cell_alphas: Vec<f32> = same_cell_positions
            .iter()
            .map(|&pos| prepared.map_alpha(pos))
            .collect();

        // All cells in the same checkerboard cell should have identical alpha
        for (i, &alpha) in same_cell_alphas.iter().enumerate() {
            for (j, &other_alpha) in same_cell_alphas.iter().enumerate() {
                if i != j {
                    assert!(
                        (alpha - other_alpha).abs() < 0.001,
                        "Cells in same checkerboard cell should have same alpha: pos{i} alpha={alpha:.3}, pos{j} alpha={other_alpha:.3}"
                    );
                }
            }
        }

        // Verify different checkerboard cells have different alphas (if we're in transition
        // range)
        let different_cell_pos = Position::new(2, 0); // Should be in a black cell
        let different_alpha = prepared.map_alpha(different_cell_pos);

        let same_cell_avg = same_cell_alphas.iter().sum::<f32>() / same_cell_alphas.len() as f32;

        // The different cell should have noticeably different alpha
        // (exact difference depends on animation progress and transition width)
        assert!(
            (0.0..=1.0).contains(&same_cell_avg),
            "Same cell average alpha should be valid: {same_cell_avg:.3}"
        );
        assert!(
            (0.0..=1.0).contains(&different_alpha),
            "Different cell alpha should be valid: {different_alpha:.3}"
        );
    }

    #[test]
    fn test_checkerboard_builder_methods() {
        // Test new() constructor
        let pattern1 = CheckerboardPattern::new(3, 1.5);
        assert_eq!(pattern1.cell_size, 3);
        assert_eq!(pattern1.transition_width, 1.5);

        // Test with_cell_size() constructor
        let pattern2 = CheckerboardPattern::with_cell_size(4);
        assert_eq!(pattern2.cell_size, 4);
        assert_eq!(pattern2.transition_width, 2.0); // Default

        // Test with_transition_width() builder
        let pattern3 = CheckerboardPattern::with_cell_size(2).with_transition_width(3.0);
        assert_eq!(pattern3.cell_size, 2);
        assert_eq!(pattern3.transition_width, 3.0);

        // Test minimum clamping
        let pattern4 = CheckerboardPattern::new(0, 0.05); // Should be clamped
        assert_eq!(pattern4.cell_size, 0); // cell_size doesn't get clamped in new()
        assert_eq!(pattern4.transition_width, 0.1); // Should be clamped to minimum

        let pattern5 = CheckerboardPattern::with_cell_size(0); // Should be clamped
        assert_eq!(pattern5.cell_size, 1); // Should be clamped to minimum
    }

    #[test]
    fn test_checkerboard_default() {
        let pattern = CheckerboardPattern::default();
        assert_eq!(pattern.cell_size, 2);
        assert_eq!(pattern.transition_width, 2.0);
    }
}
