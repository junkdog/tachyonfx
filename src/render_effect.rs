use ratatui_core::{buffer::Buffer, layout::Rect, terminal::Frame};

use crate::{Duration, Effect};

/// Trait for rendering an effect into a buffer or frame.
pub trait EffectRenderer<T> {
    /// Processes the effect for `last_tick` duration and renders it into `area`.
    fn render_effect(&mut self, effect: &mut T, area: Rect, last_tick: Duration);
}

impl EffectRenderer<Effect> for Frame<'_> {
    fn render_effect(&mut self, effect: &mut Effect, area: Rect, last_tick: Duration) {
        // render_effect(effect, self.buffer_mut(), area, last_tick);
        effect.process(last_tick, self.buffer_mut(), area);
    }
}

impl EffectRenderer<Effect> for Buffer {
    fn render_effect(&mut self, effect: &mut Effect, area: Rect, last_tick: Duration) {
        effect.process(last_tick, self, area);
    }
}
