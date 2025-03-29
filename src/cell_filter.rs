use crate::color_ext::ToRgbComponents;
use crate::{ref_count, RefCount, ThreadSafetyMarker};
use ratatui::buffer::Cell;
use ratatui::layout;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::prelude::Color;
use std::fmt;

#[cfg(not(feature = "sendable"))]
type CellPredFn = RefCount<dyn Fn(&Cell) -> bool>;
#[cfg(feature = "sendable")]
type CellPredFn = RefCount<dyn Fn(&Cell) -> bool + Send>;

#[cfg(not(feature = "sendable"))]
type PositionFnType = RefCount<dyn Fn(Position) -> bool>;
#[cfg(feature = "sendable")]
type PositionFnType = RefCount<dyn Fn(Position) -> bool + Send>;

/// A filter mode that enables effects to operate on specific cells based on various criteria.
///
/// `CellFilter` provides a flexible way to select cells for applying effects based on their
/// properties such as colors, position, content, or custom predicates. Filters can be combined
/// using logical operations to create complex selection patterns.
#[derive(Clone, Default)]
pub enum CellFilter {
    /// Selects every cell
    #[default]
    All,
    /// Selects cells within the specified area
    Area(Rect),
    /// Selects cells with matching foreground color
    FgColor(Color),
    /// Selects cells with matching background color
    BgColor(Color),
    /// Selects cells within the inner margin of the area
    Inner(Margin),
    /// Selects cells outside the inner margin of the area
    Outer(Margin),
    /// Selects cells with text
    Text,
    /// Selects cells that match all the given filters
    AllOf(Vec<CellFilter>),
    /// Selects cells that match any of the given filters
    AnyOf(Vec<CellFilter>),
    /// Selects cells that do not match any of the given filters
    NoneOf(Vec<CellFilter>),
    /// Negates the given filter
    Not(Box<CellFilter>),
    /// Selects cells within the specified layout, denoted by the index
    Layout(layout::Layout, u16),
    /// Selects cells by predicate function
    PositionFn(PositionFnType),
    /// Selects cells by predicate function
    EvalCell(CellPredFn),
}

impl CellFilter {
    /// Creates a new cell filter using a custom evaluation function.
    ///
    /// The provided function should return `true` for cells that should be selected
    /// and `false` for cells that should be excluded.
    ///
    /// # Arguments
    /// * `f` - A function that takes a reference to a Cell and returns a boolean
    ///
    /// # Type Parameters
    /// * `F` - A function type that implements the required thread safety markers
    pub fn eval_cell<F>(f: F) -> Self
        where F: Fn(&Cell) -> bool + ThreadSafetyMarker + 'static
    {
        CellFilter::EvalCell(ref_count(f))
    }

    /// Converts the filter to a human-readable string representation.
    ///
    /// This method is useful for debugging and logging purposes, providing
    /// a clear visualization of the filter's structure and parameters.
    ///
    /// # Returns
    /// A String representing the filter in a readable format
    pub fn to_string(&self) -> String {
        fn to_hex(c: &Color) -> String {
            let (r, g, b) = c.to_rgb();
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }

        fn format_margin(m: &Margin) -> String {
            format!("{}:{}", m.horizontal, m.vertical)
        }

        fn to_string(filters: &[CellFilter]) -> String {
            filters.iter()
                .map(CellFilter::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        }

        match self {
            CellFilter::All             => "all".to_string(),
            CellFilter::Area(area)      => format!("area({})", area),
            CellFilter::FgColor(color)  => format!("fg({})", to_hex(color)),
            CellFilter::BgColor(color)  => format!("bg({})", to_hex(color)),
            CellFilter::Inner(m)        => format!("inner({})", format_margin(m)),
            CellFilter::Outer(m)        => format!("outer({})", format_margin(m)),
            CellFilter::Text            => "text".to_string(),
            CellFilter::AllOf(filters)  => format!("all_of({})", to_string(filters)),
            CellFilter::AnyOf(filters)  => format!("any_of({})", to_string(filters)),
            CellFilter::NoneOf(filters) => format!("none_of({})", to_string(filters)),
            CellFilter::Not(filter)     => format!("!{}", filter.to_string()),
            CellFilter::Layout(_, idx)  => format!("layout({})", idx),
            CellFilter::PositionFn(_)   => "position_fn".to_string(),
            CellFilter::EvalCell(_)     => "eval_cell".to_string(),
        }
    }
}

/// A predicate that evaluates cells based on their position and properties using a specified filter strategy.
///
/// `CellPredicate` is created internally by `CellFilter`'s `selector` method and serves as the
/// evaluation engine for cell filtering operations. It combines spatial awareness (via a rectangular area)
/// with content-based filtering rules to determine which cells should be included in operations.
///
/// See also [crate::Shader::cell_iter].
pub struct CellPredicate {
    /// The effective area for cell evaluation after applying any area-modifying filters.
    /// This may be different from the original area if the filter modifies spatial bounds
    /// (e.g., margins or layout sections).
    filter_area: Rect,

    /// The filter strategy that defines the criteria cells must meet to be considered valid.
    /// This strategy can combine multiple filters using logical operations (AND, OR, NOT)
    /// and can include both position-based and content-based criteria.
    strategy: CellFilter,
}

impl CellPredicate {
    /// Creates a new `CellPredicate` with the specified area and filter strategy.
    ///
    /// The provided area may be modified based on the filter strategy (e.g., for margin-based filters).
    ///
    /// # Arguments
    /// * `area` - The initial rectangular area for cell evaluation
    /// * `strategy` - The filter strategy to apply
    fn new(area: Rect, strategy: CellFilter) -> Self {
        let filter_area = Self::resolve_area(area, &strategy);

        Self { filter_area, strategy }
    }

    fn resolve_area(area: Rect, mode: &CellFilter) -> Rect {
        match mode {
            CellFilter::All                 => area,
            CellFilter::Area(r)             => area.intersection(*r),
            CellFilter::Inner(margin)       => area.inner(*margin),
            CellFilter::Outer(margin)       => area.inner(*margin),
            CellFilter::Text                => area,
            CellFilter::AllOf(_)            => area,
            CellFilter::AnyOf(_)            => area,
            CellFilter::NoneOf(_)           => area,
            CellFilter::Not(m)              => Self::resolve_area(area, m.as_ref()),
            CellFilter::FgColor(_)          => area,
            CellFilter::BgColor(_)          => area,
            CellFilter::Layout(layout, idx) => layout.split(area)[*idx as usize],
            CellFilter::PositionFn(_)       => area,
            CellFilter::EvalCell(_)         => area,
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
            CellFilter::All           => true,
            CellFilter::Area(_)       => self.filter_area.contains(pos),
            CellFilter::Layout(_, _)  => self.filter_area.contains(pos),
            CellFilter::Inner(_)      => self.filter_area.contains(pos),
            CellFilter::Outer(_)      => !self.filter_area.contains(pos),
            CellFilter::Text          => {
                let ch = cell.symbol().chars().next().unwrap();
                ch.is_alphabetic() || ch.is_numeric() || " ?!.,:;()".contains(ch)
            },
            CellFilter::AllOf(s)      => s.iter()
                .all(|mode| mode.selector(self.filter_area).is_valid(pos, cell)),
            CellFilter::AnyOf(s)      => s.iter()
                .any(|mode| mode.selector(self.filter_area).is_valid(pos, cell)),
            CellFilter::NoneOf(s)     => s.iter()
                .all(|mode| !mode.selector(self.filter_area).is_valid(pos, cell)),
            CellFilter::Not(m)        => !m.selector(self.filter_area).is_valid(pos, cell),
            // CellFilter::Not(m)        => !self.valid_position(pos, m.as_ref()),
            CellFilter::FgColor(c)    => cell.fg == *c,
            CellFilter::BgColor(c)    => cell.fg == *c,
            CellFilter::PositionFn(f) => {
                #[cfg(not(feature = "sendable"))]
                return f.borrow()(pos);
                #[cfg(feature = "sendable")]
                return f.lock().unwrap()(pos);
            },
            CellFilter::EvalCell(f)   => {
                #[cfg(not(feature = "sendable"))]
                return f.borrow()(cell);
                #[cfg(feature = "sendable")]
                return f.lock().unwrap()(cell);
            },
        }
    }
}

impl CellFilter {
    pub fn selector(&self, area: Rect) -> CellPredicate {
        CellPredicate::new(area, self.clone())
    }
}

impl fmt::Debug for CellFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellFilter::All            => write!(f, "All"),
            CellFilter::Area(area)     => write!(f, "Area({:})", area),
            CellFilter::FgColor(color) => write!(f, "FgColor({:?})", color),
            CellFilter::BgColor(color) => write!(f, "BgColor({:?})", color),
            CellFilter::Inner(margin)  => write!(f, "Inner({:?})", margin),
            CellFilter::Outer(margin)  => write!(f, "Outer({:?})", margin),
            CellFilter::Text           => write!(f, "Text"),
            CellFilter::AllOf(filters) => {
                f.debug_tuple("AllOf")
                    .field(filters)
                    .finish()
            },
            CellFilter::AnyOf(filters) => {
                f.debug_tuple("AnyOf")
                    .field(filters)
                    .finish()
            },
            CellFilter::NoneOf(filters) => {
                f.debug_tuple("NoneOf")
                    .field(filters)
                    .finish()
            },
            CellFilter::Not(filter) => {
                f.debug_tuple("Not")
                    .field(filter)
                    .finish()
            },
            CellFilter::Layout(layout, idx) => {
                write!(f, "Layout({:?}, {})", layout, idx)
            },
            CellFilter::PositionFn(_) => write!(f, "PositionFn(<function>)"),
            CellFilter::EvalCell(_)   => write!(f, "EvalCell(<function>)"),
        }
    }
}

impl PartialEq for CellFilter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellFilter::All, CellFilter::All) => true,
            (CellFilter::Area(r1), CellFilter::Area(r2)) => r1 == r2,
            (CellFilter::FgColor(c1), CellFilter::FgColor(c2)) => c1 == c2,
            (CellFilter::BgColor(c1), CellFilter::BgColor(c2)) => c1 == c2,
            (CellFilter::Inner(m1), CellFilter::Inner(m2)) => m1 == m2,
            (CellFilter::Outer(m1), CellFilter::Outer(m2)) => m1 == m2,
            (CellFilter::Text, CellFilter::Text) => true,
            (CellFilter::AllOf(f1), CellFilter::AllOf(f2)) => f1 == f2,
            (CellFilter::AnyOf(f1), CellFilter::AnyOf(f2)) => f1 == f2,
            (CellFilter::NoneOf(f1), CellFilter::NoneOf(f2)) => f1 == f2,
            (CellFilter::Not(f1), CellFilter::Not(f2)) => f1 == f2,
            (CellFilter::Layout(l1, i1), CellFilter::Layout(l2, i2)) => l1 == l2 && i1 == i2,
            (CellFilter::PositionFn(_), CellFilter::PositionFn(_)) => true,
            (CellFilter::EvalCell(_), CellFilter::EvalCell(_)) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::effect_fn;
    use crate::{Duration, EffectRenderer};
    use layout::Layout;
    use ratatui::buffer::Buffer;
    use ratatui::style::Style;
    use ratatui::text::Span;

    #[test]
    fn test_cell_filter_to_string() {
        let filter = CellFilter::FgColor(Color::Red);
        assert_eq!(filter.to_string(), "fg(#800000)");

        let filter = CellFilter::BgColor(Color::Green);
        assert_eq!(filter.to_string(), "bg(#008000)");

        let filter = CellFilter::Inner(Margin::new(1, 1));
        assert_eq!(filter.to_string(), "inner(1:1)");

        let filter = CellFilter::Outer(Margin::new(3, 4));
        assert_eq!(filter.to_string(), "outer(3:4)");

        let filter = CellFilter::Text;
        assert_eq!(filter.to_string(), "text");

        let filter = CellFilter::AllOf(vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::BgColor(Color::Green),
        ]);
        assert_eq!(filter.to_string(), "all_of(fg(#800000), bg(#008000))");

        let filter = CellFilter::AnyOf(vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::BgColor(Color::Green),
        ]);
        assert_eq!(filter.to_string(), "any_of(fg(#800000), bg(#008000))");

        let filter = CellFilter::NoneOf(vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::BgColor(Color::Green),
        ]);
        assert_eq!(filter.to_string(), "none_of(fg(#800000), bg(#008000))");

        let filter = CellFilter::Not(Box::new(CellFilter::FgColor(Color::Red)));
        assert_eq!(filter.to_string(), "!fg(#800000)");

        let filter = CellFilter::Layout(Layout::horizontal(&[]), 0);
        assert_eq!(filter.to_string(), "layout(0)");

        let filter = CellFilter::PositionFn(ref_count(|_| true));
        assert_eq!(filter.to_string(), "position_fn");

        let filter = CellFilter::EvalCell(ref_count(|_| true));
        assert_eq!(filter.to_string(), "eval_cell");
    }

    #[test]
    fn test_cell_filter_eval() {
        let empty = Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
        ]);
        let fx = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_symbol("X");
            }
        });

        let mut buf = empty.clone();
        let filter = CellFilter::eval_cell(|cell| cell.symbol() == ".");

        let area = buf.area().clone();
        buf.render_effect(&mut fx.clone().with_filter(filter), area, Duration::from_millis(16));

        assert_eq!(buf, Buffer::with_lines([
            "X X X X ",
            "X X X X ",
            "X X X X ",
            "X X X X ",
        ]));

        let mut buf = empty.clone();
        let filter = CellFilter::Not(Box::new(CellFilter::Area(Rect::new(0, 0, 8, 2))));
        buf.render_effect(&mut fx.clone().with_filter(filter), area, Duration::from_millis(16));

        assert_eq!(buf, Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            "XXXXXXXX",
            "XXXXXXXX",
        ]));
    }

    #[test]
    fn test_all_any_and_none_of() {
        fn assert_filter(
            buf: &Buffer,
            filter: CellFilter,
            expected: Buffer,
        ) {
            let mut mark_fx = effect_fn((), 1, |_, _, cells| {
                for (_, c) in cells {
                    c.set_symbol("X");
                }
            }).with_filter(filter);

            let mut clear_styling = effect_fn((), 1, |_, _, cells| {
                for (_, c) in cells {
                    c.set_style(Style::reset());
                }
            });

            let mut b = buf.clone();
            b.render_effect(&mut mark_fx, buf.area, Duration::from_millis(16));
            b.render_effect(&mut clear_styling, buf.area, Duration::from_millis(16));

            assert_eq!(b, expected);
        }

        let red = Style::default().fg(Color::Red);

        let mut buf = Buffer::filled(Rect::new(0, 0, 6, 4), Cell::new("."));
        // 2nd row from top has red fg color
        buf.set_span(0, 1, &Span::from("......").style(red), 6);
        let buf = buf;

        let filters = vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::Inner(Margin::new(1, 1)),
        ];

        assert_filter(&buf, CellFilter::AllOf(filters.clone()), Buffer::with_lines([
            "......",
            ".XXXX.",
            "......",
            "......",
        ]));
        assert_filter(&buf, CellFilter::AnyOf(filters.clone()), Buffer::with_lines([
            "......",
            "XXXXXX",
            ".XXXX.",
            "......",
        ]));
        assert_filter(&buf, CellFilter::NoneOf(filters.clone()), Buffer::with_lines([
            "XXXXXX",
            "......",
            "X....X",
            "XXXXXX",
        ]));
    }
}