use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};

pub struct CellIterator<'a> {
    current: u16,
    area: Rect,
    buf: &'a mut Buffer,
}

impl<'a> CellIterator<'a> {
    pub fn new(buf: &'a mut Buffer, area: Rect) -> Self {
        Self { current: 0, area, buf }
    }
    
    fn cell_mut(&mut self) -> (Position, &mut Cell) {
        let x = self.current % self.area.width;
        let y = self.current / self.area.width;

        let pos = Position::new(self.area.x + x, self.area.y + y);
        let cell = self.buf.get_mut(pos.x, pos.y);
        (pos, cell)
    }
}

impl<'a> Iterator for CellIterator<'a> {
    type Item = (Position, &'a mut Cell);

    fn next(&mut self) -> Option<Self::Item> {
        let item = if self.current < self.area.area() {
            let (pos, cell) = self.cell_mut();
            // enforce cell's lifetime. this is safe because `buf` is guaranteed to outlive `'a`
            let cell: &'a mut Cell = unsafe { std::mem::transmute(cell) };
            
            Some((pos, cell))
        } else {
            None
        };
        
        if item.is_some() {
            self.current += 1;
        }
        
        item
    }
}