#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString};
use ratatui::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::{
    fx::sliding_window_alpha::SlidingWindowAlpha,
    pattern::{InstancedPattern, Pattern, PreparedPattern},
    Motion,
};

#[derive(Clone, Debug, Copy, PartialEq)]
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

    fn for_frame(self, global_alpha: f32, area: Rect) -> PreparedPattern<SlidingWindowAlpha, Self>
    where
        Self: Sized,
    {
        // Apply the same timer flipping logic used by slide effects
        // to compensate for the semantic mismatch in SlidingWindowAlpha
        let adjusted_progress =
            if self.direction.flips_timer() { 1.0 - global_alpha } else { global_alpha };

        PreparedPattern {
            pattern: self,
            context: SlidingWindowAlpha::builder()
                .direction(self.direction)
                .progress(adjusted_progress)
                .area(area)
                .gradient_len(self.gradient_span)
                .build(),
        }
    }
}

impl InstancedPattern for PreparedPattern<SlidingWindowAlpha, SweepPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        self.context.alpha(pos)
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for SweepPattern {
    fn dsl_format(&self) -> CompactString {
        match self.direction {
            Motion::LeftToRight => {
                format_compact!("SweepPattern::left_to_right({})", self.gradient_span)
            },
            Motion::RightToLeft => {
                format_compact!("SweepPattern::right_to_left({})", self.gradient_span)
            },
            Motion::UpToDown => format_compact!("SweepPattern::up_to_down({})", self.gradient_span),
            Motion::DownToUp => format_compact!("SweepPattern::down_to_up({})", self.gradient_span),
        }
    }
}
