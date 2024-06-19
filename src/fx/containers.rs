use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect};
use crate::{CellIterator, EffectTimer};
use crate::effect::{Effect, CellFilter};
use crate::shader::Shader;

#[derive(Default, Clone)]
pub struct SequentialEffect {
    effects: Vec<Effect>,
    current: usize,
}

#[derive(Default, Clone)]
pub struct ParallelEffect {
    effects: Vec<Effect>,
}

impl SequentialEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects, current: 0 }
    }
}

impl ParallelEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects }
    }
}

impl Shader for ParallelEffect {
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let mut remaining = Some(duration);

        for effect in self.effects.iter_mut().filter(|e| e.running()) {
            let effect_area = effect.area().unwrap_or(area);
            match effect.process(duration, buf, effect_area) {
                None => remaining = None,
                Some(d) if remaining.is_some() => {
                    remaining = Some(d.min(remaining.unwrap()));
                }
                _ => (),
            }
        }

        remaining
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

    fn done(&self) -> bool {
        self.effects.iter().all(Effect::done)
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        None
    }

    fn set_area(&mut self, area: Rect) {
        self.effects.iter_mut().for_each(|e| e.set_area(area));
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.effects.iter_mut().for_each(|e| e.set_cell_selection(strategy.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        None
    }
}

impl Shader for SequentialEffect {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {

        let mut remaining = Some(duration);
        while remaining.is_some() && !self.done() {
            let effect = &mut self.effects[self.current];
            let effect_area = effect.area().unwrap_or(area);
            remaining = effect.process(remaining.unwrap(), buf, effect_area);

            if effect.done() {
                self.current += 1;
            }
        }

        remaining
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

    fn done(&self) -> bool {
        self.current >= self.effects.len()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        None
    }

    fn set_area(&mut self, area: Rect) {
        self.effects.iter_mut().for_each(|e| e.set_area(area));
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.effects.iter_mut().for_each(|e| e.set_cell_selection(strategy.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> { None }

    fn cell_selection(&self) -> Option<CellFilter> { None }
}
