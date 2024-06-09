use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;


use crate::effect::CellFilter;
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;

#[derive(Clone)]
pub struct Sleep {
    timer: EffectTimer,
}

impl Sleep {
    pub fn new<T: Into<EffectTimer>>(duration: T) -> Self {
        Self { timer: duration.into() }
    }
}

impl Shader for Sleep {
    fn process(
        &mut self,
        duration: Duration,
        _buf: &mut Buffer,
        _area: Rect
    ) -> Option<Duration> {
        self.timer.process(duration)
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn cell_selection(&mut self, _strategy: CellFilter) {}
}


