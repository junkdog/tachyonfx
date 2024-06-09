use std::time::Duration;
use rand::{Rng, thread_rng};
use rand::prelude::{SeedableRng, SmallRng};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::effect::CellFilter;
use crate::effect_timer::EffectTimer;

use crate::shader::Shader;

#[derive(Clone)]
pub struct Dissolve {
    lifetime: EffectTimer,
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
            lifetime,
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
     fn process(
         &mut self,
         duration: Duration,
         buf: &mut Buffer,
         area: Rect
     ) -> Option<Duration> {
         let a = self.lifetime.alpha();
         let remainder = self.lifetime.process(duration);

         let cell_filter = self.cell_filter.selector(area);
         
         self.cell_iter(buf, area)
             .filter(|(pos, cell)| cell_filter.is_valid(*pos, cell))
             .enumerate()
             .filter(|(idx, _)| self.is_cell_idx_active(*idx, a))
             .for_each(|(_, (_, c))| { c.set_char(' '); });

         remainder
     }

     fn done(&self) -> bool {
          self.lifetime.done()
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

    fn cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.clone().reversed();
    }
}