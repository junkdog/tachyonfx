use ratatui_core::{
    buffer::Cell,
    layout::{Position, Rect},
};

use crate::CellFilter;

/// A predicate that evaluates cells based on their position and properties using a
/// specified filter strategy.
///
/// `CellPredicate` is created internally by `CellFilter`'s `predicate` method and serves
/// as the evaluation engine for cell filtering operations. It combines spatial awareness
/// (via a rectangular area) with content-based filtering rules to determine which cells
/// should be included in operations.
///
/// See also [crate::Shader::cell_iter].
pub struct CellPredicate<'a> {
    /// The effective area for cell evaluation after applying any area-modifying filters.
    /// This may be different from the original area if the filter modifies spatial bounds
    /// (e.g., margins or layout sections).
    filter_area: Rect,

    /// The filter strategy that defines the criteria cells must meet to be considered
    /// valid. This strategy can combine multiple filters using logical operations
    /// (AND, OR, NOT) and can include both position-based and content-based criteria.
    strategy: &'a CellFilter,
}

impl<'a> CellPredicate<'a> {
    /// Creates a new `CellPredicate` with the specified area and filter strategy.
    ///
    /// The provided area may be modified based on the filter strategy (e.g., for
    /// margin-based filters).
    ///
    /// # Arguments
    /// * `area` - The initial rectangular area for cell evaluation
    /// * `strategy` - The filter strategy to apply
    pub(crate) fn new(area: Rect, strategy: &'a CellFilter) -> Self {
        let filter_area = Self::resolve_area(area, strategy);

        Self { filter_area, strategy }
    }

    fn resolve_area(area: Rect, mode: &CellFilter) -> Rect {
        match mode {
            CellFilter::All => area,
            CellFilter::Area(r) => area.intersection(*r),
            CellFilter::RefArea(ref_rect) => area.intersection(ref_rect.get()),
            CellFilter::Inner(margin) => area.inner(*margin),
            CellFilter::Outer(margin) => area.inner(*margin),
            CellFilter::Text => area,
            CellFilter::NonEmpty => area,
            CellFilter::AllOf(_) => area,
            CellFilter::AnyOf(_) => area,
            CellFilter::NoneOf(_) => area,
            CellFilter::Not(m) => Self::resolve_area(area, m.as_ref()),
            CellFilter::FgColor(_) => area,
            CellFilter::BgColor(_) => area,
            CellFilter::Layout(layout, idx) => layout.split(area)[*idx as usize],
            CellFilter::PositionFn(_) => area,
            CellFilter::EvalCell(_) => area,
            CellFilter::Static(filter) => Self::resolve_area(area, filter.as_ref()),
        }
    }

    /// Determines if a cell at the given position meets the filter criteria.
    ///
    /// This method combines position-based and cell-content-based filtering to make
    /// the final determination.
    ///
    /// # Arguments
    /// * `pos` - The position to evaluate
    /// * `cell` - The cell at the given position
    ///
    /// # Returns
    /// `true` if the cell meets all filter criteria, `false` otherwise
    pub fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        match &self.strategy {
            CellFilter::All => true,
            CellFilter::Area(_) => self.filter_area.contains(pos),
            CellFilter::RefArea(_) => self.filter_area.contains(pos),
            CellFilter::Layout(_, _) => self.filter_area.contains(pos),
            CellFilter::Inner(_) => self.filter_area.contains(pos),
            CellFilter::Outer(_) => !self.filter_area.contains(pos),
            CellFilter::Text => {
                let ch = cell.symbol().chars().next().unwrap();
                ch.is_alphabetic() || ch.is_numeric() || " ?!.,:;()".contains(ch)
            },
            CellFilter::NonEmpty => cell.symbol() != " ",
            CellFilter::AllOf(s) => s.iter().all(|mode| {
                mode.predicate(self.filter_area)
                    .is_valid(pos, cell)
            }),
            CellFilter::AnyOf(s) => s.iter().any(|mode| {
                mode.predicate(self.filter_area)
                    .is_valid(pos, cell)
            }),
            CellFilter::NoneOf(s) => s.iter().all(|mode| {
                !mode
                    .predicate(self.filter_area)
                    .is_valid(pos, cell)
            }),
            CellFilter::Not(m) => !m.predicate(self.filter_area).is_valid(pos, cell),
            CellFilter::FgColor(c) => cell.fg == *c,
            CellFilter::BgColor(c) => cell.bg == *c,
            CellFilter::PositionFn(f) => {
                #[cfg(not(feature = "sendable"))]
                return f.borrow()(pos);
                #[cfg(feature = "sendable")]
                return f.lock().unwrap()(pos);
            },
            CellFilter::EvalCell(f) => {
                #[cfg(not(feature = "sendable"))]
                return f.borrow()(cell);
                #[cfg(feature = "sendable")]
                return f.lock().unwrap()(cell);
            },
            CellFilter::Static(f) => f.predicate(self.filter_area).is_valid(pos, cell),
        }
    }
}
