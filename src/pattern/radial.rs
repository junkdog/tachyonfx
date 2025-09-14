use ratatui::layout::{Position, Rect};

use crate::pattern::{util::ProgressionMapper, InstancedPattern, Pattern, PatternForFrame};

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
    pub fn with_transition(center: (f32, f32), transition_width: f32) -> Self {
        let (center_x, center_y) = center;
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
    pub fn with_center(mut self, center: (f32, f32)) -> Self {
        let (center_x, center_y) = center;
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

        // Calculate center position in cell coordinates
        let center_x = area.x as f32 + (pattern.center_x * area.width as f32);
        let center_y = area.y as f32 + (pattern.center_y * area.height as f32);

        // Calculate distance from center in cell coordinates
        let dx = pos.x as f32 - center_x;
        let dy = pos.y as f32 - center_y;
        let distance = (dx * dx + 2.0 * dy * 2.0 * dy).sqrt();

        // Calculate maximum radius (distance to farthest corner)
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
                    (dx * dx + dy * dy).sqrt()
                })
                .fold(0.0f32, f32::max)
        };

        let transition_radius = pattern.transition_width * max_radius;

        // Circle radius scales from -transition_radius to max_radius + transition_radius
        // This ensures at alpha=0, even the transition zone is "before" position 0
        let circle_radius =
            (global_alpha * (max_radius + 2.0 * transition_radius)) - transition_radius;

        if distance <= circle_radius {
            // Inside the solid circle - fully active
            1.0
        } else if distance <= circle_radius + transition_radius {
            // Inside the transition zone - gradient from 1.0 to 0.0
            let distance_into_transition = distance - circle_radius;
            let progress = 1.0 - (distance_into_transition / transition_radius);
            progress.clamp(0.0, 1.0)
        } else {
            // Outside the circle + transition
            0.0
        }
    }
}
