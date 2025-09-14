use ratatui::layout::{Position, Rect};

use crate::pattern::{InstancedPattern, Pattern, PatternForFrame};

#[derive(Clone, Debug, Copy)]
pub struct CheckerboardPattern {
    cell_size: u16,
    transition_width: f32,
}

impl CheckerboardPattern {
    /// Creates a checkerboard pattern with specified cell size and transition width.
    ///
    /// # Arguments
    /// * `cell_size` - Size of each checkerboard cell in terminal cells (minimum 1)
    /// * `transition_width` - Width of gradient transition between cells (0.01-1.0)
    pub fn new(cell_size: u16, transition_width: f32) -> Self {
        Self { cell_size, transition_width }
    }

    /// Creates a checkerboard pattern with custom cell size and default transition width.
    ///
    /// # Arguments
    /// * `cell_size` - Size of each checkerboard cell in terminal cells (automatically
    ///   clamped to minimum 1)
    pub fn with_cell_size(cell_size: u16) -> Self {
        Self { cell_size: cell_size.max(1), transition_width: 0.1 }
    }

    /// Sets the transition width for gradient smoothing between cells.
    ///
    /// # Arguments
    /// * `width` - Width of gradient transition zone (0.01-1.0, automatically clamped)
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.clamp(0.01, 1.0);
        self
    }

    fn is_white_cell(&self, x: u16, y: u16) -> bool {
        let cell_x = x / self.cell_size;
        let cell_y = y / self.cell_size;
        (cell_x + cell_y) % 2 == 0
    }
}

impl Default for CheckerboardPattern {
    fn default() -> Self {
        Self::new(2, 0.1)
    }
}

impl Pattern for CheckerboardPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized,
    {
        PatternForFrame { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PatternForFrame<(f32, Rect), CheckerboardPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let pattern = &self.pattern;
        let (global_alpha, area) = self.context;

        // Calculate relative position within the area
        let rel_x = (pos.x - area.x) as u16;
        let rel_y = (pos.y - area.y) as u16;

        // Determine if this is a "white" or "black" cell in the checkerboard
        let is_white = pattern.is_white_cell(rel_x, rel_y);

        let transition_width = pattern.transition_width;

        // White cells appear first, black cells appear later
        let cell_threshold = if is_white { 0.0 } else { 0.5 };

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
