use ratatui::layout::{Position, Rect};

use crate::{
    pattern::{InstancedPattern, Pattern, PatternForFrame},
    SimpleRng,
};

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
    type Context = (f32, SimpleRng);

    fn for_frame(self, alpha: f32, _area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized,
    {
        PatternForFrame { pattern: self, context: (alpha, self.rng) }
    }
}

impl InstancedPattern for PatternForFrame<(f32, SimpleRng), CoalescePattern> {
    fn map_alpha(&mut self, _pos: Position) -> f32 {
        let threshold = self.context.1.gen_f32();
        let global_alpha = self.context.0;

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
