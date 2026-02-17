use crate::CellFilter;

/// Categorizes cell filters for performance optimization.
///
/// The filter system analyzes each [`CellFilter`] to determine whether it can be
/// pre-computed (static) or must be evaluated each frame (dynamic). This classification
/// enables significant performance optimizations by caching static filter results as
/// bitmasks.
///
/// ## Filter Classification Rules
///
/// - **Static**: Filters that depend only on cell positions and area geometry
/// - **Dynamic**: Filters that depend on cell content (colors, text, etc.)
///
/// Static filters can have their results computed once and stored as a bit vector,
/// while dynamic filters must be evaluated for each cell every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FilterType {
    /// Filter results can be pre-computed and cached as a bitmask.
    #[default]
    Static,

    /// Filter results must be computed each frame based on cell content.
    Dynamic,
}

/// Trait for analyzing filters to determine their performance characteristics.
///
/// This trait is implemented by [`CellFilter`] to enable the filtering system
/// to optimize performance by distinguishing between static (cacheable) and
/// dynamic (per-frame evaluation) filters.
pub(crate) trait FilterAnalyzer {
    /// Analyzes a filter to determine if it's static or dynamic.
    ///
    /// Returns [`FilterType::Static`] if the filter can be pre-computed,
    /// or [`FilterType::Dynamic`] if it must be evaluated each frame.
    fn analyze(&self) -> FilterType;

    /// Analyzes a collection of filters for composite filter operations.
    ///
    /// For composite operations like [`CellFilter::AllOf`], the result is
    /// static only if all constituent filters are static. Returns the
    /// "highest" filter type, where Dynamic > Static.
    ///
    /// # Arguments
    /// * `filters` - The collection of filters to analyze
    ///
    /// # Returns
    /// The most restrictive filter type among all filters in the collection
    fn analyze_composite(&self, filters: &[CellFilter]) -> FilterType;
}

impl FilterAnalyzer for CellFilter {
    fn analyze(&self) -> FilterType {
        match self {
            // cacheable filters that only depend on the area properties
            CellFilter::All
            | CellFilter::Area(_)
            | CellFilter::RefArea(_)
            | CellFilter::Inner(_)
            | CellFilter::Outer(_)
            | CellFilter::Static(_)
            | CellFilter::Layout(_, _) => FilterType::Static,

            // dynamic filter are evaluated each frame
            CellFilter::FgColor(_)
            | CellFilter::BgColor(_)
            | CellFilter::Text
            | CellFilter::NonEmpty
            | CellFilter::PositionFn(_)
            | CellFilter::EvalCell(_) => FilterType::Dynamic,

            // composite filters delegate to other filters
            CellFilter::AllOf(filters)
            | CellFilter::AnyOf(filters)
            | CellFilter::NoneOf(filters) => self.analyze_composite(filters),

            // delegate to the inner filter
            CellFilter::Not(filter) => filter.analyze(),
        }
    }

    fn analyze_composite(&self, filters: &[CellFilter]) -> FilterType {
        filters
            .iter()
            .map(Self::analyze)
            .max()
            .unwrap_or(FilterType::Static)
    }
}
