use ratatui::layout::{Position, Rect};

use crate::pattern::{InstancedPattern, Pattern, PatternForFrame};
use crate::pattern::util::ProgressionMapper;

#[derive(Clone, Debug, Copy)]
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
            transition_width: 0.1,
        }
    }

    /// Creates a radial pattern with custom center point (0.0-1.0 normalized coordinates)
    /// and default transition width
    pub fn new(center_x: f32, center_y: f32) -> Self {
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: 0.1,
        }
    }

    /// Creates a radial pattern with custom center and transition width
    ///
    /// # Arguments
    /// * `center_x` - Center X position (0.0-1.0 normalized coordinates)
    /// * `center_y` - Center Y position (0.0-1.0 normalized coordinates)
    /// * `transition_width` - Width of the gradient transition zone (0.01-1.0)
    pub fn with_transition(center_x: f32, center_y: f32, transition_width: f32) -> Self {
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: transition_width.clamp(0.01, 1.0),
        }
    }

    /// Sets the transition width for gradient smoothing
    ///
    /// # Arguments
    /// * `width` - Width of the gradient transition zone (0.01-1.0). Smaller values
    ///   create sharper edges.
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.clamp(0.01, 1.0);
        self
    }

    /// Sets a custom center point for the radial pattern
    pub fn with_center(mut self, center_x: f32, center_y: f32) -> Self {
        self.center_x = center_x.clamp(0.0, 1.0);
        self.center_y = center_y.clamp(0.0, 1.0);
        self
    }
}

impl Pattern for RadialPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized,
    {
        PatternForFrame { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PatternForFrame<(f32, Rect), RadialPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let pattern = &self.pattern;

        let (global_alpha, area) = self.context;

        let (x, y) = (pos.x, pos.y);

        // Convert position to normalized coordinates (0.0-1.0)
        let norm_x = (x - area.x) as f32 / area.width as f32;
        let norm_y = (y - area.y) as f32 / area.height as f32;

        // Calculate distance from center
        let dx = norm_x - pattern.center_x;
        let dy = norm_y - pattern.center_y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Maximum possible distance from center to any corner
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let max_distance = corners
            .iter()
            .map(|(cx, cy)| {
                let dx = cx - pattern.center_x;
                let dy = cy - pattern.center_y;
                (dx * dx + dy * dy).sqrt()
            })
            .fold(0.0f32, f32::max);

        // Create radial effect: starts from center and expands outward
        let normalized_distance = (distance / max_distance);

        // Use ProgressionMapper to handle the expanded alpha range
        // let expanded_alpha = global_alpha + self.transition_width;
        let mapper =
            ProgressionMapper::new(-pattern.transition_width, 1.0 + pattern.transition_width);
        let local_alpha = mapper.map(global_alpha);

        let result = if normalized_distance <= local_alpha {
            // Inside the expanding circle - fully active
            1.0
        } else if normalized_distance <= local_alpha + pattern.transition_width {
            // Gradient edge for smooth transition
            let edge_progress = (local_alpha + pattern.transition_width - normalized_distance)
                / pattern.transition_width;
            edge_progress
        } else {
            // Outside the expanding circle
            0.0
        };

        // Clamp final result to 0.0-1.0 range
        result.clamp(0.0, 1.0)
    }
}
