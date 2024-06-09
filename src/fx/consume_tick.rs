use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::effect::CellFilter;
use crate::shader::Shader;

/// consumes any remaining duration for a single tick.
#[derive(Default, Clone)]
pub struct ConsumeTick {
    has_consumed_tick: bool,
}

impl Shader for ConsumeTick {
    fn process(
        &mut self,
        _duration: Duration,
        _buf: &mut Buffer,
        _area: Rect,
    ) -> Option<Duration> {
        self.has_consumed_tick = true;
        None
    }

    fn done(&self) -> bool { self.has_consumed_tick }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn cell_selection(&mut self, _strategy: CellFilter) {}
}
