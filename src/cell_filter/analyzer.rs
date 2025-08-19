use crate::CellFilter;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FilterType {
    #[default]
    Static,
    Dynamic,
}

pub(crate) trait FilterAnalyzer {
    fn analyze(&self) -> FilterType;
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
            | CellFilter::Layout(_, _) => FilterType::Static,

            // dynamic filter are evaluated each frame
            CellFilter::FgColor(_)
            | CellFilter::BgColor(_)
            | CellFilter::Text
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
