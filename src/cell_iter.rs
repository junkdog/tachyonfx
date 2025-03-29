use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use crate::{CellFilter, CellPredicate};

pub struct CellIterator<'a> {
    current: u32,
    area: Rect,
    buf: &'a mut Buffer,
    filter: CellPredicate,
}

impl<'a> CellIterator<'a> {
    pub fn new(
        buf: &'a mut Buffer,
        area: Rect,
        filter: Option<CellFilter>,
    ) -> Self {
        Self {
            current: 0,
            area: area.intersection(buf.area),
            buf,
            filter: filter.unwrap_or_default().selector(area),
        }
    }

    fn cell_mut(&mut self) -> Option<(Position, &mut Cell)> {
        let x = self.current as u16 % self.area.width;
        let y = self.current as u16 / self.area.width;

        let pos = Position::new(self.area.x + x, self.area.y + y);
        let cell = self.buf.cell_mut(pos)?;
        Some((pos, cell))
    }

    fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        self.filter.is_valid(pos, cell)
    }
}

impl<'a> Iterator for CellIterator<'a> {
    type Item = (Position, &'a mut Cell);

    fn next(&mut self) -> Option<Self::Item> {
        // let selector = &self.filter;
        let area = self.area.area();
        while self.current < area {
            let (pos, cell) = self.cell_mut()?;
            // enforce cell's lifetime. this is safe because `buf` is guaranteed to outlive `'a`
            let cell: &'a mut Cell = unsafe { std::mem::transmute(cell) };
            self.current += 1;

            if self.filter.strategy == CellFilter::All || self.is_valid(pos, cell) {
                return Some((pos, cell));
            }
        }

        None
    }
}