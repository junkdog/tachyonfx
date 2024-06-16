use std::time::Duration;
use ratatui::buffer::{Buffer};
use ratatui::layout::{Rect};
use crate::cell_iter::CellIterator;

use crate::effect::CellFilter;
use crate::EffectTimer;

/// A shader-like object that can be processed for a duration.
pub trait Shader {
    /// Process the shader for the given duration. Returns any overflowed
    /// duration if the shader is done.
    ///
    /// This default implementation calls `execute` with the alpha value and the cells.
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let (overflow, alpha) = self.timer_mut()
            .map(|t| (t.process(duration), t.alpha()))
            .unwrap_or((None, 1.0));

        let requested_cells = self.cell_iter(buf, area);
        self.execute(alpha, area, requested_cells);

        overflow
    }

    /// Execute the shader with the given alpha value and cells. This is where
    /// the actual shader logic should be implemented, unless `process` is
    /// overridden.
    fn execute(
        &mut self,
        alpha: f32,
        area: Rect,
        cell_iter: CellIterator,
    );

    fn cell_iter<'a>(
        &mut self,
        buf: &'a mut Buffer,
        area: Rect,
    ) -> CellIterator<'a> {
        CellIterator::new(buf, area, self.cell_filter())
    }
    
    /// Returns true if the effect is done.
    fn done(&self) -> bool;
    fn running(&self) -> bool { !self.done() }
    fn clone_box(&self) -> Box<dyn Shader>;
    fn area(&self) -> Option<Rect>;

    fn set_area(&mut self, area: Rect);
    fn cell_selection(&mut self, strategy: CellFilter);

    fn reverse(&mut self) {}

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> { None }
    fn cell_filter(&self) -> Option<CellFilter> { None }
}
