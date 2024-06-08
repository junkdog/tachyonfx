use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::Rect;
use crate::shader::Shader;

pub trait EffectRenderer<T> {
    fn render_effect(
        &mut self,
        effect: &mut T,
        area: Rect,
        last_tick: Duration
    );
}

impl<S: Shader> EffectRenderer<S> for Frame<'_> {
    fn render_effect(
        &mut self,
        effect: &mut S,
        area: Rect,
        last_tick: Duration,
    ) {
        render_effect(effect, self.buffer_mut(), area, last_tick);
    }
}


impl<S: Shader> EffectRenderer<S> for Buffer {
    fn render_effect(
        &mut self,
        effect: &mut S,
        area: Rect,
        last_tick: Duration,
    ) {
        render_effect(effect, self, area, last_tick);
    }
}

fn render_effect<S: Shader>(
    // effect: &mut Effect,
    effect: &mut S,
    buf: &mut Buffer,
    area: Rect,
    last_tick: Duration,
) {
    effect.process(
        last_tick,
        buf,
        area
    );
}
