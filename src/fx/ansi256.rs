use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};

use crate::{CellIterator, Duration};
use crate::color_ext::AsIndexedColor;
use crate::color_mapper::ColorMapper;
use crate::CellFilter;
use crate::shader::Shader;

#[derive(Clone, Default)]
pub struct Ansi256 {
    area: Option<Rect>,
}

impl Shader for Ansi256 {
    fn name(&self) -> &'static str {
        "ansi256"
    }

    fn process(
        &mut self,
        _duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        let safe_area = area.intersection(buf.area);
        for y in area.top()..safe_area.bottom() {
            for x in area.left()..safe_area.right() {
                let cell = buf.cell_mut(Position::new(x, y))?;
                let fg = fg_mapper.map(cell.fg, 0.0, |c| c.as_indexed_color());
                let bg = bg_mapper.map(cell.bg, 0.0, |c| c.as_indexed_color());

                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        None
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // handled by process
    }

    fn done(&self) -> bool { false }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {}

    fn reset(&mut self) {}
}