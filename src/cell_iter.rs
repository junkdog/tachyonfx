use core::mem;

use ratatui::{
    buffer::{Buffer, Cell},
    layout::{Position, Rect},
};

use crate::{
    cell_filter::{CellValidator, FilterProcessor},
    CellFilter,
};

/// An iterator over terminal cells within a rectangular area.
///
/// `CellIterator` provides efficient access to terminal cells in a [`Buffer`] within a
/// specified rectangular region. It supports optional filtering via [`CellFilter`] to
/// selectively process cells based on their properties.
///
/// ## Performance Considerations
///
/// For optimal performance, **prefer [`for_each_cell`](Self::for_each_cell) over
/// iterator-based iteration** when you need to process all cells without using iterator
/// combinators. The `for_each_cell` method avoids division and modulo operations, making
/// it significantly faster.
///
/// Use the [`Iterator`] trait implementation only when you need iterator combinators like
/// [`filter`](Iterator::filter), [`map`](Iterator::map), or [`take`](Iterator::take).
///
/// ## Examples
///
/// ### Preferred: Using `for_each_cell` (fastest)
/// ```rust
/// use ratatui::{buffer::Buffer, layout::Rect};
/// use tachyonfx::CellIterator;
///
/// let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
/// let mut iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 10, 5), None);
///
/// iter.for_each_cell(|pos, cell| {
///     // Process each cell - this is the fastest approach
///     cell.set_char('X');
/// });
/// ```
///
/// ### Using iterator when combinators are needed
/// ```rust
/// use ratatui::{buffer::Buffer, layout::{Position, Rect}};
/// use tachyonfx::CellIterator;
///
/// let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
/// let mut iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 10, 5), None);
///
/// // Use iterator when you need combinators
/// for (pos, cell) in iter.filter(|(pos, _)| pos.x % 2 == 0) {
///     cell.set_char('O');
/// }
/// ```
pub struct CellIterator<'a> {
    current: u32,
    area: Rect,
    buf: &'a mut Buffer,
    predicate: Option<CellValidator<'a>>,
}

impl<'a> CellIterator<'a> {
    /// Creates a new `CellIterator` over the specified area of a buffer.
    ///
    /// The iterator will process cells within the intersection of the provided `area` and
    /// the buffer's bounds. If a `cell_filter` is provided, only cells matching the
    /// filter criteria will be yielded.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable reference to the terminal buffer
    /// * `area` - The rectangular area to iterate over
    /// * `filter_processor` - Optional filter processor to apply to cells (use `None` for
    ///   no filtering)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::{buffer::Buffer, layout::Rect};
    /// use tachyonfx::CellIterator;
    ///
    /// let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
    ///
    /// // Create iterator without filtering
    /// let iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 5, 3), None);
    /// ```
    pub fn new(
        buf: &'a mut Buffer,
        area: Rect,
        filter_processor: Option<&'a FilterProcessor>,
    ) -> Self {
        Self {
            current: 0,
            area: area.intersection(buf.area),
            buf,
            predicate: filter_processor
                .filter(|p| p.filter_ref() != &CellFilter::All) // all is same as no filter
                .map(|f| f.validator()),
        }
    }

    /// Applies a function to each cell in the iterator's area.
    ///
    /// This is the **preferred method** for iterating over cells when you don't need
    /// iterator combinators. It's significantly faster than using the `Iterator`
    /// trait because it avoids division and modulo operations used for coordinate
    /// calculation.
    ///
    /// The function receives the cell's position and a mutable reference to the cell,
    /// allowing for efficient in-place modifications.
    ///
    /// # Performance
    ///
    /// This method is optimized for speed and should be used unless you specifically need
    /// iterator combinators like `filter`, `map`, or `take`.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes `(Position, &mut Cell)` and processes each cell
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::{buffer::Buffer, layout::Rect, style::Color};
    /// use tachyonfx::CellIterator;
    ///
    /// let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
    /// let iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 10, 5), None);
    ///
    /// iter.for_each_cell(|pos, cell| {
    ///     // Set different colors based on position
    ///     if pos.x % 2 == 0 {
    ///         cell.set_fg(Color::Red);
    ///     } else {
    ///         cell.set_fg(Color::Blue);
    ///     }
    ///     cell.set_char('█');
    /// });
    /// ```
    pub fn for_each_cell<F>(self, mut f: F)
    where
        F: FnMut(Position, &mut Cell),
    {
        let area = self.area;
        let predicate = self.predicate.as_ref();

        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let pos = Position::new(x, y);
                if let Some(cell) = self.buf.cell_mut(pos) {
                    if predicate.is_none_or(|p| p.is_valid(pos, cell)) {
                        f(pos, cell);
                    }
                }
            }
        }
    }

    fn cell_mut(&mut self) -> Option<(Position, &mut Cell)> {
        // calculate x/y using u32 arithmetic to avoid truncation when current > u16::MAX
        let x = (self.current % self.area.width as u32) as u16;
        let y = (self.current / self.area.width as u32) as u16;

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
            let cell: &'a mut Cell = unsafe { mem::transmute(cell) };
            self.current += 1;

            if self
                .predicate
                .as_ref()
                .is_none_or(|p| p.is_valid(pos, cell))
            {
                return Some((pos, cell));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::{vec, vec::Vec};

    use ratatui::{buffer::Buffer, layout::Rect, style::Color};

    use super::*;

    #[test]
    fn test_normal_iteration() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        buffer[(0, 0)].set_char('A');
        buffer[(1, 0)].set_char('B');
        buffer[(2, 0)].set_char('C');
        buffer[(0, 1)].set_char('D');
        buffer[(1, 1)].set_char('E');
        buffer[(2, 1)].set_char('F');

        let mut iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 3, 2), None);
        let mut positions = Vec::new();
        let mut chars = Vec::new();

        for (pos, cell) in &mut iter {
            positions.push(pos);
            chars.push(cell.symbol().chars().next().unwrap_or(' '));
        }

        assert_eq!(positions.len(), 6);
        assert_eq!(chars, vec!['A', 'B', 'C', 'D', 'E', 'F']);
        assert_eq!(positions[0], Position::new(0, 0));
        assert_eq!(positions[1], Position::new(1, 0));
        assert_eq!(positions[2], Position::new(2, 0));
        assert_eq!(positions[3], Position::new(0, 1));
        assert_eq!(positions[4], Position::new(1, 1));
        assert_eq!(positions[5], Position::new(2, 1));
    }

    #[test]
    fn test_for_each_cell() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        buffer[(0, 0)].set_char('A');
        buffer[(1, 0)].set_char('B');
        buffer[(2, 0)].set_char('C');
        buffer[(0, 1)].set_char('D');
        buffer[(1, 1)].set_char('E');
        buffer[(2, 1)].set_char('F');

        let iter = CellIterator::new(&mut buffer, Rect::new(0, 0, 3, 2), None);
        let mut positions = Vec::new();
        let mut chars = Vec::new();

        iter.for_each_cell(|pos, cell| {
            positions.push(pos);
            chars.push(cell.symbol().chars().next().unwrap_or(' '));
            // Modify the cell to test mutability
            cell.set_fg(Color::Red);
        });

        assert_eq!(positions.len(), 6);
        assert_eq!(chars, vec!['A', 'B', 'C', 'D', 'E', 'F']);
        assert_eq!(positions[0], Position::new(0, 0));
        assert_eq!(positions[1], Position::new(1, 0));
        assert_eq!(positions[2], Position::new(2, 0));
        assert_eq!(positions[3], Position::new(0, 1));
        assert_eq!(positions[4], Position::new(1, 1));
        assert_eq!(positions[5], Position::new(2, 1));

        // Verify cells were modified
        assert_eq!(buffer[(0, 0)].fg, Color::Red);
        assert_eq!(buffer[(1, 0)].fg, Color::Red);
        assert_eq!(buffer[(2, 0)].fg, Color::Red);
    }
}
