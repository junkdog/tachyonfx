use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use crate::CellFilter;

pub struct CellIterator<'a> {
    current: u16,
    area: Rect,
    buf: &'a mut Buffer,
    filter: Option<CellFilter>,
}

impl<'a> CellIterator<'a> {
    pub fn new(
        buf: &'a mut Buffer,
        area: Rect,
        filter: Option<CellFilter>,
    ) -> Self {
        Self { current: 0, area, buf, filter }
    }

    fn cell_mut(&mut self) -> Option<(Position, &mut Cell)> {
        let x = self.current % self.area.width;
        let y = self.current / self.area.width;

        let pos = Position::new(self.area.x + x, self.area.y + y);
        let cell = self.buf.cell_mut(pos)?;
        Some((pos, cell))
    }
}

impl<'a> Iterator for CellIterator<'a> {
    type Item = (Position, &'a mut Cell);

    fn next(&mut self) -> Option<Self::Item> {
        let selector = self.filter.as_ref().map(|f| f.selector(self.area));
        let area = self.area.area();
        while self.current < area {
            let (pos, cell) = self.cell_mut()?;
            // enforce cell's lifetime. this is safe because `buf` is guaranteed to outlive `'a`
            let cell: &'a mut Cell = unsafe { std::mem::transmute(cell) };
            self.current += 1;

            if let Some(filter) = &selector {
                if filter.is_valid(pos, cell) {
                    return Some((pos, cell));
                }
            } else {
                return Some((pos, cell));
            }
        }

        None
    }
}