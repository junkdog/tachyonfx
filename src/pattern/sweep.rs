use ratatui::layout::{Position, Rect};

use crate::{
    fx::sliding_window_alpha::SlidingWindowAlpha,
    pattern::{InstancedPattern, Pattern, PatternForFrame},
    Motion,
};

#[derive(Clone, Debug, Copy)]
pub struct SweepPattern {
    direction: Motion,
    gradient_span: u16,
}

impl SweepPattern {
    pub fn left_to_right(gradient_span: u16) -> Self {
        Self { direction: Motion::LeftToRight, gradient_span }
    }

    pub fn right_to_left(gradient_span: u16) -> Self {
        Self { direction: Motion::RightToLeft, gradient_span }
    }

    pub fn up_to_down(gradient_span: u16) -> Self {
        Self { direction: Motion::UpToDown, gradient_span }
    }

    pub fn down_to_up(gradient_span: u16) -> Self {
        Self { direction: Motion::DownToUp, gradient_span }
    }

    /// Creates a sweep pattern with custom gradient span
    ///
    /// # Arguments
    /// * `direction` - The direction of the slide
    /// * `gradient_span` - The relative width of the gradient (0.1 = sharp, 0.5 = wide)
    pub fn new(direction: Motion, gradient_span: u16) -> Self {
        Self { direction, gradient_span }
    }
}

impl Pattern for SweepPattern {
    type Context = SlidingWindowAlpha;

    fn for_frame(self, global_alpha: f32, area: Rect) -> PatternForFrame<SlidingWindowAlpha, Self>
    where
        Self: Sized,
    {
        PatternForFrame {
            pattern: self,
            context: SlidingWindowAlpha::builder()
                .direction(self.direction)
                .progress(global_alpha)
                .area(area)
                .gradient_len(self.gradient_span)
                .build(),
        }
    }
}

impl InstancedPattern for PatternForFrame<SlidingWindowAlpha, SweepPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        self.context.alpha(pos)
    }
}
