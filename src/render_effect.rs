use ratatui::{buffer::Buffer, layout::Rect, Frame};

use crate::{Duration, Effect};

pub trait EffectRenderer<T> {
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
