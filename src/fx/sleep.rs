
use ratatui::layout::Rect;
use crate::CellIterator;


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
    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
         // slept
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

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        None
    }
}


