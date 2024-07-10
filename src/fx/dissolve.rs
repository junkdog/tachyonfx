use rand::{Rng, thread_rng};
use rand::prelude::{SeedableRng, SmallRng};
use ratatui::layout::Rect;

use crate::CellIterator;
use crate::effect::CellFilter;
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;

#[derive(Clone)]
pub struct Dissolve {
    timer: EffectTimer,
    cyclic_cell_activation: Vec<f32>,
    area: Option<Rect>,
    cell_filter: CellFilter,
}

impl Dissolve {
    pub fn new(
        lifetime: EffectTimer,
        cell_cycle: usize,
    ) -> Self {
        let mut rng = SmallRng::from_rng(thread_rng()).unwrap();

        Self {
            timer: lifetime,
            cyclic_cell_activation: (0..cell_cycle).map(|_| rng.gen_range(0.0..1.0)).collect(),
            area: None,
            cell_filter: CellFilter::All,
        }
    }

    fn is_cell_idx_active(&self, idx: usize, a: f32) -> bool {
        a > self.cyclic_cell_activation[idx % self.cyclic_cell_activation.len()]
    }
}

impl Shader for Dissolve {

    fn execute(&mut self, alpha: f32, _area: Rect, cell_iter: CellIterator) {
        cell_iter.enumerate()
            .filter(|(idx, _)| self.is_cell_idx_active(*idx, alpha))
            .for_each(|(_, (_, c))| { c.set_char(' '); });
    }

    fn done(&self) -> bool {
          self.timer.done()
     }

     fn clone_box(&self) -> Box<dyn Shader> {
          Box::new(self.clone())
     }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area)
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}