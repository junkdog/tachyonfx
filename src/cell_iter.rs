use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use crate::{CellFilter, CellPredicate};

pub struct CellIterator<'a> {
    current: u32,
    area: Rect,
    buf: &'a mut Buffer,
    predicate: Option<CellPredicate>,
}

impl<'a> CellIterator<'a> {
    pub fn new(
        buf: &'a mut Buffer,
        area: Rect,
        cell_filter: Option<CellFilter>,
    ) -> Self {
        Self {
            current: 0,
            area: area.intersection(buf.area),
            buf,
            predicate: cell_filter
                .filter(|f| *f != CellFilter::All)
                .map(|f| f.selector(area)),
        }
    }

    fn cell_mut(&mut self) -> Option<(Position, &mut Cell)> {
        let x = self.current as u16 % self.area.width;
        let y = self.current as u16 / self.area.width;

        let pos = Position::new(self.area.x + x, self.area.y + y);
        let cell = self.buf.cell_mut(pos)?;
        Some((pos, cell))
    }
}

impl<'a> Iterator for CellIterator<'a> {
    type Item = (Position, &'a mut Cell);

    fn next(&mut self) -> Option<Self::Item> {
        let area = self.area.area();
        while self.current < area {
            let (pos, cell) = self.cell_mut()?;
            // enforce cell's lifetime. this is safe because `buf` is guaranteed to outlive `'a`
            let cell: &'a mut Cell = unsafe { std::mem::transmute(cell) };
            self.current += 1;

            if let Some(predicate) = &self.predicate {
                if predicate.is_valid(pos, cell) {
                    return Some((pos, cell));
                }
            } else {
                return Some((pos, cell));
            }
        }

        None
    }
}