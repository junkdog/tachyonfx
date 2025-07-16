use ratatui::{buffer::Buffer, layout::Rect, Frame};

use crate::{shader::Shader, Duration};

pub trait EffectRenderer<T> {
    fn render_effect(&mut self, effect: &mut T, area: Rect, last_tick: Duration);
}

impl<S: Shader> EffectRenderer<S> for Frame<'_> {
    fn render_effect(&mut self, effect: &mut S, area: Rect, last_tick: Duration) {
        render_effect(effect, self.buffer_mut(), area, last_tick);
    }
}

impl<S: Shader> EffectRenderer<S> for Buffer {
    fn render_effect(&mut self, effect: &mut S, area: Rect, last_tick: Duration) {
        render_effect(effect, self, area, last_tick);
    }
}

fn render_effect<S: Shader>(effect: &mut S, buf: &mut Buffer, area: Rect, last_tick: Duration) {
    effect.process(last_tick, buf, area);
}
