use alloc::vec::Vec;
use core::ops::Not;

use ratatui_core::{
    buffer::{Buffer, Cell},
    layout::{Position, Rect},
};

use crate::{
    bitvec::BitVec,
    cell_filter::{analyzer::FilterType, FilterAnalyzer},
    CellFilter, CellPredicate, RefRect,
};

/// High-level processor that optimizes filter evaluation based on filter characteristics.
///
/// The `FilterProcessor` serves as the main entry point for filter processing,
/// automatically choosing between static (pre-computed bitmask) and dynamic (per-cell
/// evaluation) strategies based on the filter's analysis result.
///
/// ## Performance Optimization Strategy
///
/// - **Static filters**: Pre-computed as bit vectors for O(1) lookup per cell
/// - **Dynamic filters**: Evaluated per-cell using [`CellPredicate`] for maximum
///   flexibility
///
/// This optimization can provide significant performance improvements, especially for
/// complex static filters that would otherwise require expensive per-cell evaluation.
#[derive(Debug, Clone)]
pub enum FilterProcessor {
    /// Optimized processor for static filters using pre-computed bitmasks.
    ///
    /// Contains a [`StaticFilterProcessor`] that has pre-computed which cells
    /// match the filter criteria, allowing for O(1) validation per cell.
    Static(StaticFilterProcessor),

    /// Direct evaluation processor for dynamic filters.
    ///
    /// Stores the original [`CellFilter`] and area for per-cell evaluation
    /// using [`CellPredicate`]. Required for filters that depend on cell content.
    Dynamic(CellFilter, Rect),
}

impl FilterProcessor {
    /// Creates a [`CellPredicate`] for evaluating cells against the processed filter.
    ///
    /// This method provides a unified interface regardless of whether the filter
    /// is processed statically or dynamically. The returned predicate can be used
    /// to test individual cells against the filter criteria.
    ///
    /// # Arguments
    /// * `area` - The rectangular area for cell evaluation
    ///
    /// # Returns
    /// A [`CellPredicate`] configured for the specified area
    #[deprecated(since = "0.19.0", note = "use validator() instead")]
    pub fn predicate(&self, area: Rect) -> CellPredicate<'_> {
        match self {
            FilterProcessor::Static(processor) => processor.filter.predicate(area),
            FilterProcessor::Dynamic(filter, _) => filter.predicate(area),
        }
    }

    /// Creates a new `FilterProcessor` for the given filter, automatically choosing
    /// the optimal processing strategy.
    ///
    /// The processor analyzes the filter to determine if it can be statically
    /// optimized (pre-computed as a bitmask) or requires dynamic evaluation.
    ///
    /// # Arguments
    /// * `filter` - The [`CellFilter`] to process
    ///
    /// # Returns
    /// A `FilterProcessor` configured with the optimal strategy
    pub(crate) fn new(filter: CellFilter) -> Self {
        match filter.analyze() {
            FilterType::Static => FilterProcessor::Static(StaticFilterProcessor::new(filter)),
            FilterType::Dynamic => FilterProcessor::Dynamic(filter, Rect::default()),
        }
    }

    /// Updates the processor with a new area, potentially triggering recomputation
    /// of static filter bitmasks.
    ///
    /// For static processors, this may cause expensive recomputation if the area
    /// has changed significantly. For dynamic processors, this simply updates the
    /// stored area for future predicate creation.
    ///
    /// # Arguments
    /// * `area` - The new rectangular area for filter evaluation
    pub(crate) fn update(&mut self, buf: &Buffer, area: Rect) {
        match self {
            FilterProcessor::Static(processor) => processor.update(buf, area),
            FilterProcessor::Dynamic(_, a) => *a = area,
        }
    }

    /// Creates a [`CellValidator`] for efficient cell-by-cell validation.
    ///
    /// The validator provides an optimized interface for checking if individual
    /// cells match the filter criteria, automatically using the most efficient
    /// validation strategy based on the filter type.
    ///
    /// # Returns
    /// A [`CellValidator`] configured for optimal performance
    pub fn validator(&self) -> CellValidator<'_> {
        match self {
            FilterProcessor::Static(processor) => CellValidator::Static(processor),
            FilterProcessor::Dynamic(filter, area) => {
                CellValidator::Dynamic(filter.predicate(*area))
            },
        }
    }

    /// Returns a reference to the underlying [`CellFilter`].
    ///
    /// This method provides access to the original filter regardless of how
    /// it's being processed internally.
    ///
    /// # Returns
    /// A reference to the underlying [`CellFilter`]
    pub fn filter_ref(&self) -> &CellFilter {
        match self {
            FilterProcessor::Static(processor) => &processor.filter,
            FilterProcessor::Dynamic(filter, _) => filter,
        }
    }
}

/// Optimized validator for efficiently checking individual cells against filter criteria.
///
/// `CellValidator` provides a unified interface for cell validation while automatically
/// using the most efficient validation strategy based on the filter type. It abstracts
/// over the difference between static (bitmask-based) and dynamic (predicate-based)
/// validation methods.
pub enum CellValidator<'a> {
    /// Validator using pre-computed static filter bitmask for O(1) validation.
    Static(&'a StaticFilterProcessor),

    /// Validator using dynamic predicate evaluation for content-dependent filters.
    Dynamic(CellPredicate<'a>),
}

impl CellValidator<'_> {
    /// Determines if a cell at the given position meets the filter criteria.
    pub fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        match self {
            CellValidator::Static(processor) => processor.is_valid(pos),
            CellValidator::Dynamic(predicate) => predicate.is_valid(pos, cell),
        }
    }
}

/// Optimized processor for static filters using pre-computed bitmasks.
///
/// `StaticFilterProcessor` provides significant performance improvements for filters
/// that depend only on cell positions and area geometry. It pre-computes which cells
/// match the filter criteria and stores the results as a bit vector, enabling O(1)
/// cell validation.
///
/// ## Supported Filter Types
///
/// Static processing works for filters that depend only on geometry:
/// - [`CellFilter::All`], [`CellFilter::Area`], [`CellFilter::Inner`],
///   [`CellFilter::Outer`]
/// - [`CellFilter::Layout`] (layout-based selections)
/// - [`CellFilter::RefArea`] (with dynamic area tracking)
/// - Logical combinations of static filters ([`CellFilter::AllOf`],
///   [`CellFilter::AnyOf`], etc.)
///
/// ## Memory Usage
///
/// The bitmask requires 1 bit per cell in the area, so memory usage is:
/// `area.width * area.height` bits, or roughly `area.width * area.height / 8` bytes.
/// For a typical 80x24 terminal area, this uses about 240 bytes.
#[derive(Debug, Clone)]
pub struct StaticFilterProcessor {
    /// The original filter being processed.
    ///
    /// Stored for predicate creation and debugging purposes. The actual filtering
    /// logic is pre-computed and stored in the bitmask.
    filter: CellFilter,

    /// Pre-computed bitmask indicating which cells match the filter.
    ///
    /// Each bit corresponds to a cell position in row-major order:
    /// `index = y * area_width + x`. A `true` bit indicates the cell at that
    /// position matches the filter criteria.
    cell_indices: BitVec,

    /// The area for which the current bitmask was computed.
    ///
    /// Used to determine when recomputation is necessary due to area changes.
    /// When the area changes, the entire bitmask must be recalculated.
    last_active_area: Rect,

    /// Cached RefRect values for change detection.
    ///
    /// Stores the `(area, RefRect)` pairs that were used during the last bitmask
    /// computation. When any RefRect value changes, the bitmask must be recomputed
    /// to reflect the new geometry.
    ref_rects: Vec<(Rect, RefRect)>,
}

impl From<CellFilter> for FilterProcessor {
    fn from(value: CellFilter) -> Self {
        FilterProcessor::new(value)
    }
}

impl StaticFilterProcessor {
    /// Creates a new static filter processor for the given filter.
    ///
    /// The processor initializes with an empty bitmask that will be computed
    /// on the first call to [`update`](Self::update). RefRect dependencies
    /// are analyzed and cached for change detection.
    ///
    /// # Arguments
    /// * `filter` - The static filter to be processed
    ///
    /// # Returns
    /// A new `StaticFilterProcessor` ready for area updates
    fn new(filter: CellFilter) -> Self {
        let ref_rects = find_ref_rects(&filter);

        Self {
            filter,
            cell_indices: BitVec::new(),
            last_active_area: Rect::default(),
            ref_rects,
        }
    }

    /// Validates if a cell at the given position matches the filter criteria.
    ///
    /// Performs O(1) validation by looking up the position in the pre-computed
    /// bitmask. The position is converted to a bit index using row-major ordering.
    ///
    /// # Arguments
    /// * `pos` - The position to validate
    ///
    /// # Returns
    /// `true` if the cell at the position matches the filter, `false` otherwise
    ///
    /// # Panics
    /// This method assumes the bitmask has been computed via [`update`](Self::update).
    /// Using it before calling `update` may result in incorrect behavior.
    fn is_valid(&self, pos: Position) -> bool {
        let row_offset = pos.y as usize * self.last_active_area.right() as usize;
        self.is_valid_index(row_offset + pos.x as usize)
    }

    /// Updates the processor for a new area, recomputing the bitmask if necessary.
    ///
    /// The bitmask is only recomputed if:
    /// - The area dimensions or position have changed
    /// - Any RefRect dependencies have been modified
    ///
    /// This method can be expensive for large areas with complex filters, as it
    /// must evaluate the filter for every cell position.
    ///
    /// # Arguments
    /// * `area` - The new area for filter processing
    fn update(&mut self, buf: &Buffer, area: Rect) {
        if self.requires_resize(area) {
            self.cell_indices = calculate_cell_indices(buf, area, &self.filter);
            self.ref_rects = find_ref_rects(&self.filter);
            self.last_active_area = area;
        }
    }

    /// Validates if a bit index in the bitmask indicates a matching cell.
    ///
    /// Internal method that performs bounds checking and bitmask lookup.
    /// The index should be computed as `y * area_width + x`.
    ///
    /// # Arguments
    /// * `index` - The bit index to check
    ///
    /// # Returns
    /// `true` if the index is valid and the bit is set, `false` otherwise
    fn is_valid_index(&self, index: usize) -> bool {
        if index >= self.cell_indices.len() {
            return false; // Out of bounds
        }

        self.cell_indices.get(index)
    }

    /// Determines if the bitmask needs to be recomputed for the given area.
    ///
    /// Checks for area changes and [`RefRect`] modifications that would invalidate
    /// the current bitmask.
    ///
    /// # Arguments
    /// * `area` - The area to check against cached state
    ///
    /// # Returns
    /// `true` if recomputation is needed, `false` if the current bitmask is valid
    fn requires_resize(&self, area: Rect) -> bool {
        let area_changed = area != self.last_active_area;

        for (rect, ref_rect) in &self.ref_rects {
            if rect != &ref_rect.get() {
                return true; // Area has changed, need to recalculate
            }
        }

        area_changed
    }
}

/// Recursively finds all RefRect dependencies within a filter tree.
fn find_ref_rects(filter: &CellFilter) -> Vec<(Rect, RefRect)> {
    let mut ref_rects = Vec::new();

    match filter {
        CellFilter::RefArea(ref_rect) => {
            ref_rects.push((ref_rect.get(), ref_rect.clone()));
        },
        CellFilter::AllOf(filters) | CellFilter::AnyOf(filters) | CellFilter::NoneOf(filters) => {
            for sub_filter in filters {
                ref_rects.extend(find_ref_rects(sub_filter));
            }
        },
        CellFilter::Not(filter) => {
            ref_rects.extend(find_ref_rects(filter.as_ref()));
        },
        CellFilter::Static(filter) => {
            ref_rects.extend(find_ref_rects(filter.as_ref()));
        },
        _ => {}, // Other filters do not have ref rects
    }

    ref_rects
}

/// Computes a bitmask indicating which cells match the given static filter.
fn calculate_cell_indices(buf: &Buffer, area: Rect, filter: &CellFilter) -> BitVec {
    let size = area.right() as usize * area.bottom() as usize;
    let mut cell_indices = BitVec::new();
    cell_indices.resize(size, false);

    let mut activate_area = |r: Rect, v: bool| {
        for y in r.y..r.bottom() {
            let row_offset = y as usize * area.right() as usize;
            for x in r.x..r.right() {
                let x = x as usize;
                let index = row_offset + x;
                cell_indices.set(index, v);
            }
        }
    };

    match &filter {
        CellFilter::All => activate_area(area, true),
        CellFilter::Area(r) => activate_area(*r, true),
        CellFilter::RefArea(r) => activate_area(r.get().intersection(area), true),
        CellFilter::Inner(m) => activate_area(area.inner(*m), true),
        CellFilter::Outer(m) => {
            activate_area(area, true);
            activate_area(area.inner(*m), false);
        },
        CellFilter::Layout(l, idx) => {
            let sub_area = l.split(area)[*idx as usize];
            activate_area(sub_area, true);
        },

        CellFilter::AllOf(filters) => {
            let all_of = filters
                .iter()
                .map(|f| calculate_cell_indices(buf, area, f))
                .reduce(core::ops::BitAnd::bitand);

            if let Some(indices) = all_of {
                cell_indices = indices;
            }
        },
        CellFilter::AnyOf(filters) => {
            let any_of = filters
                .iter()
                .map(|f| calculate_cell_indices(buf, area, f))
                .reduce(core::ops::BitOr::bitor);

            if let Some(indices) = any_of {
                cell_indices = indices;
            }
        },
        CellFilter::NoneOf(filters) => {
            let none_of = filters
                .iter()
                .map(|f| calculate_cell_indices(buf, area, f))
                .reduce(core::ops::BitOr::bitor)
                .map(core::ops::Not::not);

            if let Some(indices) = none_of {
                cell_indices = indices;
            }
        },
        CellFilter::Not(f) => cell_indices = calculate_cell_indices(buf, area, f).not(),

        CellFilter::Static(filter) => {
            // Static variant wrapping another filter - delegate to the wrapped filter
            // This should only be called for Static(static_filter) since
            // Static(dynamic_filter) uses the dynamic processor
            cell_indices = calculate_cell_indices(buf, area, filter.as_ref());
        },

        // these are dynamic filters that can be precomputed when wrapped in Static
        CellFilter::FgColor(_)
        | CellFilter::BgColor(_)
        | CellFilter::Text
        | CellFilter::NonEmpty
        | CellFilter::PositionFn(_)
        | CellFilter::EvalCell(_) => {
            let pred = CellPredicate::new(area, filter);
            for y in area.y..area.bottom() {
                let row_offset = y as usize * area.right() as usize;
                for x in area.x..area.right() {
                    let index = row_offset + x as usize;

                    let position: Position = (x, y).into();
                    if let Some(cell) = buf.cell(position) {
                        cell_indices.set(index, pred.is_valid(position, cell));
                    }
                }
            }
        },
    };

    cell_indices
}
