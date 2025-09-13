use ratatui::layout::Rect;
use crate::{Motion, SimpleRng};

pub(crate) trait Pattern {
    /// Transform global alpha (0.0-1.0) to local alpha for a specific cell position
    /// 
    /// # Arguments
    /// * `global_alpha` - The overall effect progress (0.0 = start, 1.0 = complete)
    /// * `pos` - Cell position (x, y) within the buffer
    /// * `area` - The rectangular area being processed
    /// 
    /// # Returns
    /// Local alpha value (0.0-1.0) for this cell at this time
    fn transform_alpha(&mut self, global_alpha: f32, pos: (u16, u16), area: Rect) -> f32;
}

#[derive(Clone, Debug, Copy)]
pub struct SlidePattern {
    direction: Motion,
    gradient_span: f32,
}

impl SlidePattern {
    pub fn left_to_right() -> Self {
        Self { direction: Motion::LeftToRight, gradient_span: 0.7 }
    }
    
    pub fn right_to_left() -> Self {
        Self { direction: Motion::RightToLeft, gradient_span: 0.7 }
    }
    
    pub fn up_to_down() -> Self {
        Self { direction: Motion::UpToDown, gradient_span: 0.7 }
    }
    
    pub fn down_to_up() -> Self {
        Self { direction: Motion::DownToUp, gradient_span: 0.7 }
    }
    
    /// Creates a slide pattern with custom gradient span
    /// 
    /// # Arguments
    /// * `direction` - The direction of the slide
    /// * `gradient_span` - The relative width of the gradient (0.1 = sharp, 0.5 = wide)
    pub fn new(direction: Motion, gradient_span: f32) -> Self {
        Self { direction, gradient_span: gradient_span.max(0.01) }
    }
    
    /// Sets a custom gradient span for smoother or sharper transitions
    pub fn with_gradient_span(mut self, span: f32) -> Self {
        self.gradient_span = span.max(0.01);
        self
    }
}

impl Pattern for SlidePattern {
    fn transform_alpha(&mut self, global_alpha: f32, pos: (u16, u16), area: Rect) -> f32 {
        let (x, y) = pos;
        
        let normalized_pos = match self.direction {
            Motion::LeftToRight => {
                let relative_x = (x - area.x) as f32;
                let width = area.width as f32;
                relative_x / width
            }
            Motion::RightToLeft => {
                let relative_x = (area.right() - 1 - x) as f32;
                let width = area.width as f32;
                relative_x / width
            }
            Motion::UpToDown => {
                let relative_y = (y - area.y) as f32;
                let height = area.height as f32;
                relative_y / height
            }
            Motion::DownToUp => {
                let relative_y = (area.bottom() - 1 - y) as f32;
                let height = area.height as f32;
                relative_y / height
            }
        };
        
        // Calculate alpha based on global progress and position
        // This creates a sliding gradient effect
        let alpha = (global_alpha * (1.0 + self.gradient_span) - normalized_pos) / self.gradient_span;
        
        alpha.clamp(0.0, 1.0)
    }
}
struct RadialPattern;
struct DiagonalPattern;
struct CheckerboardPattern;

#[derive(Clone, Debug, Copy, Default)]
pub struct CoalescePattern {
    rng: SimpleRng,
}

impl CoalescePattern {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pattern for CoalescePattern {
    fn transform_alpha(&mut self, global_alpha: f32, _pos: (u16, u16), _area: Rect) -> f32 {
        let threshold = self.rng.gen_f32();
        
        // Create a smooth transition based on how far global_alpha exceeds the threshold
        // This provides a gradient effect while still maintaining randomness per cell
        if global_alpha <= threshold {
            0.0
        } else {
            // Smooth transition from 0.0 to 1.0 based on how much we exceed the threshold
            let progress = (global_alpha - threshold) / (1.0 - threshold);
            progress.clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn slide_left_to_right() {
        let mut pattern = SlidePattern::left_to_right();
        let area = Rect::new(0, 0, 10, 5);
        
        // At start (alpha=0), no cells should be progressed
        assert_eq!(pattern.transform_alpha(0.0, (0, 0), area), 0.0);
        assert_eq!(pattern.transform_alpha(0.0, (5, 0), area), 0.0);
        assert_eq!(pattern.transform_alpha(0.0, (9, 0), area), 0.0);
        
        // At middle (alpha=0.5), there should be a gradient across the area
        // Left side should have higher alpha than right side
        let left_alpha = pattern.transform_alpha(0.5, (0, 0), area);
        let mid_alpha = pattern.transform_alpha(0.5, (5, 0), area);
        let right_alpha = pattern.transform_alpha(0.5, (9, 0), area);
        
        assert!(left_alpha > mid_alpha);
        assert!(mid_alpha > right_alpha);
        assert!(left_alpha <= 1.0);
        assert!(right_alpha >= 0.0);
        
        // At end (alpha=1), all cells should be fully progressed
        assert_eq!(pattern.transform_alpha(1.0, (0, 0), area), 1.0);
        assert_eq!(pattern.transform_alpha(1.0, (5, 0), area), 1.0);
        assert_eq!(pattern.transform_alpha(1.0, (9, 0), area), 1.0);
    }

    #[test]
    fn slide_right_to_left() {
        let mut pattern = SlidePattern::right_to_left();
        let area = Rect::new(0, 0, 10, 5);
        
        // At middle (alpha=0.5), right side should have higher alpha than left side
        let right_alpha = pattern.transform_alpha(0.5, (9, 0), area);
        let mid_alpha = pattern.transform_alpha(0.5, (5, 0), area);
        let left_alpha = pattern.transform_alpha(0.5, (0, 0), area);
        
        assert!(right_alpha > mid_alpha);
        assert!(mid_alpha > left_alpha);
    }

    #[test]
    fn slide_up_to_down() {
        let mut pattern = SlidePattern::up_to_down();
        let area = Rect::new(0, 0, 5, 10);
        
        // At middle (alpha=0.5), top should have higher alpha than bottom
        let top_alpha = pattern.transform_alpha(0.5, (0, 0), area);
        let mid_alpha = pattern.transform_alpha(0.5, (0, 5), area);
        let bottom_alpha = pattern.transform_alpha(0.5, (0, 9), area);
        
        assert!(top_alpha > mid_alpha);
        assert!(mid_alpha > bottom_alpha);
    }

    #[test]
    fn slide_down_to_up() {
        let mut pattern = SlidePattern::down_to_up();
        let area = Rect::new(0, 0, 5, 10);
        
        // At middle (alpha=0.5), bottom should have higher alpha than top
        let bottom_alpha = pattern.transform_alpha(0.5, (0, 9), area);
        let mid_alpha = pattern.transform_alpha(0.5, (0, 5), area);
        let top_alpha = pattern.transform_alpha(0.5, (0, 0), area);
        
        assert!(bottom_alpha > mid_alpha);
        assert!(mid_alpha > top_alpha);
    }

    #[test]
    fn coalesce_pattern_returns_gradient() {
        let mut pattern = CoalescePattern::new();
        let area = Rect::new(0, 0, 10, 10);
        
        // At start (alpha=0), should return 0.0 for all positions
        assert_eq!(pattern.transform_alpha(0.0, (0, 0), area), 0.0);
        assert_eq!(pattern.transform_alpha(0.0, (5, 5), area), 0.0);
        
        // At end (alpha=1), should return 1.0 for all positions
        assert_eq!(pattern.transform_alpha(1.0, (0, 0), area), 1.0);
        assert_eq!(pattern.transform_alpha(1.0, (5, 5), area), 1.0);
        
        // At middle values, should return values between 0.0 and 1.0
        let mut has_intermediate_values = false;
        for y in 0..10 {
            for x in 0..10 {
                let alpha = pattern.transform_alpha(0.5, (x, y), area);
                assert!(alpha >= 0.0 && alpha <= 1.0);
                if alpha > 0.0 && alpha < 1.0 {
                    has_intermediate_values = true;
                }
            }
        }
        
        // With 100 cells and random thresholds, we should see some intermediate values
        assert!(has_intermediate_values, "CoalescePattern should produce gradient values between 0.0 and 1.0");
    }
}



