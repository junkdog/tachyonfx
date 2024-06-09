use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect};
use crate::cell_iter::CellIterator;

use crate::effect::CellFilter;

/// A shader-like object that can be processed for a duration.
pub trait Shader {
    /// Process the shader for the given duration. Returns any overflowed
    /// duration if the shader is done.
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration>;

    fn cell_iter<'a>(
        &mut self,
        buf: &'a mut Buffer,
        area: Rect
    ) -> CellIterator<'a> {
        CellIterator::new(buf, area)
    }
    
    /// Returns true if the effect is done.
    fn done(&self) -> bool;

    fn running(&self) -> bool { !self.done() }

    fn clone_box(&self) -> Box<dyn Shader>;
    
    fn area(&self) -> Option<Rect>;
    
    fn set_area(&mut self, area: Rect);
    fn cell_selection(&mut self, strategy: CellFilter);

    fn reverse(&mut self) {}
}
