use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::effect::{Effect, CellFilter};
use crate::shader::Shader;

#[derive(Clone)]
pub struct NeverComplete {
    effect: Effect,
}

impl NeverComplete {
    pub fn new(effect: Effect) -> Self {
        Self { effect }
    }
}

impl Shader for NeverComplete {
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.effect.process(duration, buf, area);
        None
    }

    fn done(&self) -> bool                      { false }
    fn clone_box(&self) -> Box<dyn Shader>      { Box::new(self.clone()) }
    fn area(&self) -> Option<Rect>              { self.effect.area() }
    fn set_area(&mut self, area: Rect)          { self.effect.set_area(area) }

    fn cell_selection(&mut self, strategy: CellFilter) {
        self.effect.cell_selection(strategy);
    }

    fn reverse(&mut self) {
        self.effect.reverse()
    }
}