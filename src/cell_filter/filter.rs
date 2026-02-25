use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use ratatui_core::{
    buffer::Cell,
    layout,
    layout::{Margin, Position, Rect},
    style::Color,
};

use crate::{
    color_ext::ToRgbComponents, ref_count, CellPredicate, RefCount, RefRect, ThreadSafetyMarker,
};

#[cfg(not(feature = "sendable"))]
type CellPredFn = RefCount<dyn Fn(&Cell) -> bool>;
#[cfg(feature = "sendable")]
type CellPredFn = RefCount<dyn Fn(&Cell) -> bool + Send>;

#[cfg(not(feature = "sendable"))]
type PositionFnType = RefCount<dyn Fn(Position) -> bool>;
#[cfg(feature = "sendable")]
type PositionFnType = RefCount<dyn Fn(Position) -> bool + Send>;

/// A filter mode that enables effects to operate on specific cells based on various
/// criteria.
///
/// `CellFilter` provides a flexible way to select cells for applying effects based on
/// their properties such as colors, position, content, or custom predicates. Filters can
/// be combined using logical operations to create complex selection patterns.
///
/// ## Performance Characteristics
///
/// Filters are analyzed and categorized as either **static** (cacheable) or **dynamic**
/// (evaluated per-frame) for performance optimization:
///
/// - **Static filters**: Depend only on area geometry and can be pre-computed as
///   bitmasks. Examples: [`All`](Self::All), [`Area`](Self::Area),
///   [`Inner`](Self::Inner), [`Layout`](Self::Layout)
/// - **Dynamic filters**: Depend on cell content and must be evaluated each frame.
///   Examples: [`FgColor`](Self::FgColor), [`Text`](Self::Text),
///   [`EvalCell`](Self::EvalCell)
///
/// ## Usage in Effects
///
/// Cell filters are typically applied to effects using the
/// [`Effect::with_filter`](crate::Effect::with_filter) method, allowing effects to target
/// specific subsets of cells:
///
/// ```rust
/// use tachyonfx::{fx, CellFilter};
/// use ratatui_core::style::Color;
/// use ratatui_core::layout::Margin;
///
/// // Apply fade effect only to red text
/// let effect = fx::fade_to_fg(Color::Blue, (1000, tachyonfx::Interpolation::Linear))
///     .with_filter(CellFilter::FgColor(Color::Red));
///
/// // Apply effect to border area (outer margin)
/// let border_effect = fx::dissolve(320)
///     .with_filter(CellFilter::Outer(Margin::new(1, 1)));
///
/// // Combine filters with logical operations
/// let complex_filter = CellFilter::AllOf(vec![
///     CellFilter::Text,
///     CellFilter::Inner(Margin::new(2, 1))
/// ]);
/// ```
///
/// ## Filter Composition
///
/// Filters support logical composition for complex selection patterns:
/// - [`AllOf`](Self::AllOf): All filters must match (AND operation)
/// - [`AnyOf`](Self::AnyOf): Any filter must match (OR operation)
/// - [`NoneOf`](Self::NoneOf): No filters must match (NOR operation)
/// - [`Not`](Self::Not): Inverts the filter result (NOT operation)
#[derive(Clone, Default)]
pub enum CellFilter {
    /// Selects every cell in the area (no filtering).
    ///
    /// This is the default filter that allows all effects to apply to every cell
    /// within the specified area. It's the most permissive filter and has optimal
    /// performance as it requires no evaluation.
    #[default]
    All,

    /// Selects cells within the specified rectangular area.
    ///
    /// Only cells whose positions fall within the provided [`Rect`] bounds will be
    /// selected. This filter is **static** and can be pre-computed as a bitmask
    /// for optimal performance.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::Rect;
    ///
    /// let filter = CellFilter::Area(Rect::new(5, 10, 20, 15));
    /// // Selects cells in a 20x15 rectangle starting at position (5, 10)
    /// ```
    Area(Rect),

    /// Selects cells within the area defined by a [`RefRect`](crate::RefRect).
    ///
    /// Similar to [`Area`](Self::Area) but uses a reference-counted rectangle that
    /// can be dynamically updated. This filter is **static** with respect to the
    /// current rectangle value, but can change when the RefRect is modified.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::{CellFilter, RefRect};
    /// use ratatui_core::layout::Rect;
    ///
    /// let ref_rect = RefRect::new(Rect::new(0, 0, 10, 10));
    /// let filter = CellFilter::RefArea(ref_rect.clone());
    /// // The filter area can be updated by modifying ref_rect
    /// ref_rect.set(Rect::new(5, 5, 15, 15));
    /// ```
    RefArea(RefRect),

    /// Selects cells with a matching foreground color.
    ///
    /// This filter is **dynamic** and must be evaluated each frame as it depends
    /// on the current cell content. Only cells whose foreground color exactly
    /// matches the specified color will be selected.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::style::Color;
    ///
    /// let filter = CellFilter::FgColor(Color::Red);
    /// // Selects only cells with red foreground color
    /// ```
    FgColor(Color),

    /// Selects cells with a matching background color.
    ///
    /// This filter is **dynamic** and must be evaluated each frame as it depends
    /// on the current cell content. Only cells whose background color exactly
    /// matches the specified color will be selected.
    ///
    /// # Example  
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::style::Color;
    ///
    /// let filter = CellFilter::BgColor(Color::Blue);
    /// // Selects only cells with blue background color
    /// ```
    BgColor(Color),

    /// Selects cells within the inner margin of the area.
    ///
    /// Creates an inner rectangle by applying the specified margin inward from
    /// the area boundaries. This filter is **static** and can be pre-computed.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::Margin;
    ///
    /// let filter = CellFilter::Inner(Margin::new(2, 1));
    /// // Selects cells 2 columns and 1 row inward from the edges
    /// ```
    Inner(Margin),

    /// Selects cells outside the inner margin of the area (border region).
    ///
    /// Selects all cells except those within the inner margin, effectively
    /// creating a border or frame effect. This filter is **static** and can be
    /// pre-computed.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::Margin;
    ///
    /// let filter = CellFilter::Outer(Margin::new(1, 1));
    /// // Selects cells in the outer 1-cell border around the area
    /// ```
    Outer(Margin),

    /// Selects cells containing textual content.
    ///
    /// This filter is **dynamic** and identifies cells that contain characters
    /// typically considered text: alphabetic characters, numeric characters,
    /// spaces, and common punctuation (`?!.,:;()`). Empty cells and cells with
    /// purely graphical characters are excluded.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    ///
    /// let filter = CellFilter::Text;
    /// // Selects cells containing letters, numbers, or common punctuation
    /// ```
    Text,

    /// Selects cells that contain a non-space symbol.
    ///
    /// This filter is **dynamic** and excludes cells whose symbol is `" "` (a single
    /// space), which typically represents an empty or unwritten cell. Unlike [`Text`],
    /// which matches alphanumeric characters and common punctuation, `NonEmpty` matches
    /// any cell that is not blank — including box-drawing characters, symbols, and other
    /// graphical content.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    ///
    /// let filter = CellFilter::NonEmpty;
    /// // Selects cells whose symbol is not " "
    /// ```
    NonEmpty,

    /// Selects cells that match ALL of the given filters (logical AND).
    ///
    /// A cell must satisfy every filter in the collection to be selected. The
    /// performance characteristics depend on the constituent filters - if all
    /// filters are **static**, the result can be pre-computed.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::style::Color;
    /// use ratatui_core::layout::Margin;
    ///
    /// let filter = CellFilter::AllOf(vec![
    ///     CellFilter::Text,
    ///     CellFilter::FgColor(Color::Red),
    ///     CellFilter::Inner(Margin::new(1, 1))
    /// ]);
    /// // Selects red text within the inner margin
    /// ```
    AllOf(Vec<CellFilter>),

    /// Selects cells that match ANY of the given filters (logical OR).
    ///
    /// A cell needs to satisfy only one filter in the collection to be selected.
    /// The performance characteristics depend on the constituent filters.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::style::Color;
    ///
    /// let filter = CellFilter::AnyOf(vec![
    ///     CellFilter::FgColor(Color::Red),
    ///     CellFilter::BgColor(Color::Blue)
    /// ]);
    /// // Selects cells that are either red text or have blue background
    /// ```
    AnyOf(Vec<CellFilter>),

    /// Selects cells that match NONE of the given filters (logical NOR).
    ///
    /// A cell must fail to satisfy every filter in the collection to be selected.
    /// This is equivalent to NOT(AnyOf(filters)).
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::style::Color;
    ///
    /// let filter = CellFilter::NoneOf(vec![
    ///     CellFilter::FgColor(Color::Red),
    ///     CellFilter::Text
    /// ]);
    /// // Selects cells that are neither red nor contain text
    /// ```
    NoneOf(Vec<CellFilter>),

    /// Inverts the result of the given filter (logical NOT).
    ///
    /// Selects cells that do NOT match the inner filter. The performance
    /// characteristics match those of the inner filter.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::Margin;
    ///
    /// let filter = CellFilter::Not(Box::new(
    ///     CellFilter::Inner(Margin::new(2, 2))
    /// ));
    /// // Selects all cells except those in the inner area
    /// ```
    Not(Box<CellFilter>),

    /// Selects cells within a specific section of a layout.
    ///
    /// Uses ratatui's [`Layout`](ratatui_core::layout::Layout) system to split the area
    /// and selects cells within the section specified by the index. This filter
    /// is **static** and can be pre-computed.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::{Layout, Constraint, Direction};
    ///
    /// let layout = Layout::default()
    ///     .direction(Direction::Horizontal)
    ///     .constraints([Constraint::Percentage(50), Constraint::Percentage(50)]);
    ///
    /// let filter = CellFilter::Layout(layout, 0);
    /// // Selects cells in the left half of the area
    /// ```
    Layout(layout::Layout, u16),

    /// Selects cells using a custom position-based predicate function.
    ///
    /// Provides maximum flexibility by allowing custom logic based on cell
    /// position. The function receives a [`Position`] and returns `true` for
    /// cells that should be selected. This filter is **dynamic**.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::{CellFilter, ref_count};
    /// use ratatui_core::layout::Position;
    ///
    /// let filter = CellFilter::PositionFn(ref_count(|pos: Position| {
    ///     (pos.x + pos.y) % 2 == 0  // Checkerboard pattern
    /// }));
    /// ```
    PositionFn(PositionFnType),

    /// Selects cells using a custom cell-content-based predicate function.
    ///
    /// Provides maximum flexibility by allowing custom logic based on the
    /// entire cell content. The function receives a [`Cell`] and returns `true`
    /// for cells that should be selected. This filter is **dynamic**.
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::{CellFilter, ref_count};
    /// use ratatui_core::buffer::Cell;
    ///
    /// let filter = CellFilter::eval_cell(|cell: &Cell| {
    ///     cell.symbol().len() > 1  // Multi-character symbols
    /// });
    /// ```
    EvalCell(CellPredFn),

    /// Treats a wrapped filter as static for optimization purposes, the filter is
    /// re-evaluated whenever the effect area or any referenced `RefRect`s change.
    ///
    /// ## When to Use
    ///
    /// Use this variant when:
    /// - The wrapped filter depends on cell content that won't change during the effect
    /// - You have a complex dynamic filter that's expensive to evaluate per-frame
    Static(Box<CellFilter>),
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
    where
        F: Fn(&Cell) -> bool + ThreadSafetyMarker + 'static,
    {
        CellFilter::EvalCell(ref_count(f))
    }

    /// Wraps this filter in a `Not` variant, effectively negating its selection criteria.
    pub fn negated(self) -> Self {
        CellFilter::Not(Box::new(self))
    }

    /// Wraps this filter in a [`CellFilter::Static`], treating it as static for
    /// optimization.
    ///
    /// ## Safety and Correctness
    ///
    /// This optimization is only sound when the underlying cell contents will NOT change
    /// during the lifetime of the effect that owns this filter. If cell content changes
    /// (colors, characters, styles), the cached evaluation may become incorrect.
    ///
    /// Safe to use with filters that depend only on:
    /// - Area geometry (Area, Inner, Outer, Layout)
    /// - Position-based logic that doesn't change
    /// - Cell content that remains constant during the effect
    ///
    /// Unsafe with filters on dynamic content:
    /// - Color filters when colors change during the effect
    /// - Text filters when characters change during the effect
    /// - Custom predicates that depend on mutable cell properties
    pub fn into_static(self) -> Self {
        CellFilter::Static(Box::new(self))
    }

    /// Converts the filter to a human-readable string representation.
    ///
    /// This method is useful for debugging and logging purposes, providing
    /// a clear visualization of the filter's structure and parameters.
    ///
    /// # Returns
    /// A String representing the filter in a readable format
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        fn to_hex(c: Color) -> String {
            let (r, g, b) = c.to_rgb();
            format!("#{r:02x}{g:02x}{b:02x}")
        }

        fn format_margin(m: Margin) -> String {
            format!("{}:{}", m.horizontal, m.vertical)
        }

        fn to_string(filters: &[CellFilter]) -> String {
            filters
                .iter()
                .map(CellFilter::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        }

        match self {
            CellFilter::All => "all".to_string(),
            CellFilter::Area(area) => format!("area({area})"),
            CellFilter::RefArea(ref_rect) => format!("ref_area({})", ref_rect.get()),
            CellFilter::FgColor(color) => format!("fg({})", to_hex(*color)),
            CellFilter::BgColor(color) => format!("bg({})", to_hex(*color)),
            CellFilter::Inner(m) => format!("inner({})", format_margin(*m)),
            CellFilter::Outer(m) => format!("outer({})", format_margin(*m)),
            CellFilter::Text => "text".to_string(),
            CellFilter::NonEmpty => "non_empty".to_string(),
            CellFilter::AllOf(filters) => format!("all_of({})", to_string(filters)),
            CellFilter::AnyOf(filters) => format!("any_of({})", to_string(filters)),
            CellFilter::NoneOf(filters) => format!("none_of({})", to_string(filters)),
            CellFilter::Not(filter) => format!("!{}", filter.to_string()),
            CellFilter::Layout(_, idx) => format!("layout({idx})"),
            CellFilter::PositionFn(_) => "position_fn".to_string(),
            CellFilter::EvalCell(_) => "eval_cell".to_string(),
            CellFilter::Static(filter) => format!("static({})", filter.to_string()),
        }
    }

    /// Creates a [`CellPredicate`] for efficiently evaluating cells against this filter.
    ///
    /// The predicate combines the filter strategy with a specific area to create
    /// an evaluation engine that can determine which cells match the filter criteria.
    /// This method is primarily used internally by the effect system.
    ///
    /// # Arguments
    /// * `area` - The rectangular area within which cells will be evaluated
    ///
    /// # Returns
    /// A [`CellPredicate`] that can evaluate individual cells against this filter
    ///
    /// # Example
    /// ```rust
    /// use tachyonfx::CellFilter;
    /// use ratatui_core::layout::{Rect, Position};
    /// use ratatui_core::buffer::Cell;
    ///
    /// let filter = CellFilter::Text;
    /// let area = Rect::new(0, 0, 10, 10);
    /// let predicate = filter.predicate(area);
    ///
    /// // Check if a specific cell matches the filter
    /// let cell = Cell::new("A");
    /// let pos = Position::new(5, 5);
    /// let matches = predicate.is_valid(pos, &cell);
    /// ```
    pub fn predicate(&self, area: Rect) -> CellPredicate<'_> {
        CellPredicate::new(area, self)
    }

    /// Creates a [`CellPredicate`] for efficiently evaluating cells against this filter.
    ///
    /// **Deprecated:** Use [`predicate()`](#method.predicate) instead.
    ///
    /// # Arguments
    ///
    /// * `area` - The rectangular area within which to apply the filter
    ///
    /// # Returns
    ///
    /// A [`CellPredicate`] that can evaluate individual cells against this filter
    #[deprecated(since = "0.17.0", note = "Use `predicate()` instead")]
    pub fn selector(&self, area: Rect) -> CellPredicate<'_> {
        self.predicate(area)
    }
}

impl fmt::Debug for CellFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellFilter::All => write!(f, "All"),
            CellFilter::Area(area) => write!(f, "Area({area:})"),
            CellFilter::RefArea(ref_rect) => write!(f, "RefArea({:?})", ref_rect.get()),
            CellFilter::FgColor(color) => write!(f, "FgColor({color:?})"),
            CellFilter::BgColor(color) => write!(f, "BgColor({color:?})"),
            CellFilter::Inner(margin) => write!(f, "Inner({margin:?})"),
            CellFilter::Outer(margin) => write!(f, "Outer({margin:?})"),
            CellFilter::Text => write!(f, "Text"),
            CellFilter::NonEmpty => write!(f, "NonEmpty"),
            CellFilter::AllOf(filters) => f.debug_tuple("AllOf").field(filters).finish(),
            CellFilter::AnyOf(filters) => f.debug_tuple("AnyOf").field(filters).finish(),
            CellFilter::NoneOf(filters) => f.debug_tuple("NoneOf").field(filters).finish(),
            CellFilter::Not(filter) => f.debug_tuple("Not").field(filter).finish(),
            CellFilter::Layout(layout, idx) => {
                write!(f, "Layout({layout:?}, {idx})")
            },
            CellFilter::PositionFn(_) => write!(f, "PositionFn(<function>)"),
            CellFilter::EvalCell(_) => write!(f, "EvalCell(<function>)"),
            CellFilter::Static(filter) => f.debug_tuple("Static").field(filter).finish(),
        }
    }
}

impl PartialEq for CellFilter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellFilter::All, CellFilter::All) => true,
            (CellFilter::Area(r1), CellFilter::Area(r2)) => r1 == r2,
            (CellFilter::RefArea(r1), CellFilter::RefArea(r2)) => r1.get() == r2.get(),
            (CellFilter::FgColor(c1), CellFilter::FgColor(c2)) => c1 == c2,
            (CellFilter::BgColor(c1), CellFilter::BgColor(c2)) => c1 == c2,
            (CellFilter::Inner(m1), CellFilter::Inner(m2)) => m1 == m2,
            (CellFilter::Outer(m1), CellFilter::Outer(m2)) => m1 == m2,
            (CellFilter::Text, CellFilter::Text) => true,
            (CellFilter::NonEmpty, CellFilter::NonEmpty) => true,
            (CellFilter::AllOf(f1), CellFilter::AllOf(f2)) => f1 == f2,
            (CellFilter::AnyOf(f1), CellFilter::AnyOf(f2)) => f1 == f2,
            (CellFilter::NoneOf(f1), CellFilter::NoneOf(f2)) => f1 == f2,
            (CellFilter::Not(f1), CellFilter::Not(f2)) => f1 == f2,
            (CellFilter::Layout(l1, i1), CellFilter::Layout(l2, i2)) => l1 == l2 && i1 == i2,
            (CellFilter::PositionFn(_), CellFilter::PositionFn(_)) => true,
            (CellFilter::EvalCell(_), CellFilter::EvalCell(_)) => true,
            (CellFilter::Static(f1), CellFilter::Static(f2)) => f1 == f2,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use layout::Layout;
    use ratatui_core::{buffer::Buffer, style::Style, text::Span};

    use super::*;
    use crate::{fx::effect_fn, Duration, EffectRenderer};

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

        let filter = CellFilter::Layout(
            Layout::horizontal(&[] as &[ratatui_core::layout::Constraint]),
            0,
        );
        assert_eq!(filter.to_string(), "layout(0)");

        let filter = CellFilter::PositionFn(ref_count(|_| true));
        assert_eq!(filter.to_string(), "position_fn");

        let filter = CellFilter::EvalCell(ref_count(|_| true));
        assert_eq!(filter.to_string(), "eval_cell");

        let ref_rect = RefRect::new(Rect::new(5, 10, 20, 30));
        let filter = CellFilter::RefArea(ref_rect);
        assert_eq!(filter.to_string(), "ref_area(20x30+5+10)");

        let filter = CellFilter::Static(Box::new(CellFilter::Text));
        assert_eq!(filter.to_string(), "static(text)");

        let filter = CellFilter::Static(Box::new(CellFilter::AllOf(vec![
            CellFilter::Text,
            CellFilter::FgColor(Color::Red),
        ])));
        assert_eq!(filter.to_string(), "static(all_of(text, fg(#800000)))");
    }

    #[test]
    fn test_cell_filter_eval() {
        let empty = Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . . "]);
        let fx = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_symbol("X");
            }
        });

        let mut buf = empty.clone();
        let filter = CellFilter::eval_cell(|cell| cell.symbol() == ".");

        let area = *buf.area();
        buf.render_effect(
            &mut fx.clone().with_filter(filter),
            area,
            Duration::from_millis(16),
        );

        assert_eq!(
            buf,
            Buffer::with_lines(["X X X X ", "X X X X ", "X X X X ", "X X X X ",])
        );

        let mut buf = empty;
        let filter = CellFilter::Not(Box::new(CellFilter::Area(Rect::new(0, 0, 8, 2))));
        buf.render_effect(&mut fx.with_filter(filter), area, Duration::from_millis(16));

        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", "XXXXXXXX", "XXXXXXXX",])
        );
    }

    #[test]
    fn test_all_any_and_none_of() {
        fn assert_filter(buf: &Buffer, filter: CellFilter, expected: &Buffer) {
            let mut mark_fx = effect_fn((), 1, |_, _, cells| {
                for (_, c) in cells {
                    c.set_symbol("X");
                }
            })
            .with_filter(filter);

            let mut clear_styling = effect_fn((), 1, |_, _, cells| {
                for (_, c) in cells {
                    c.set_style(Style::reset());
                }
            });

            let mut b = buf.clone();
            b.render_effect(&mut mark_fx, buf.area, Duration::from_millis(16));
            b.render_effect(&mut clear_styling, buf.area, Duration::from_millis(16));

            assert_eq!(&b, expected);
        }

        let red = Style::default().fg(Color::Red);

        let mut buf = Buffer::filled(Rect::new(0, 0, 6, 4), Cell::new("."));
        // 2nd row from top has red fg color
        buf.set_span(0, 1, &Span::from("......").style(red), 6);
        let buf = buf;

        let filters = vec![CellFilter::FgColor(Color::Red), CellFilter::Inner(Margin::new(1, 1))];

        assert_filter(
            &buf,
            CellFilter::AllOf(filters.clone()),
            &Buffer::with_lines(["......", ".XXXX.", "......", "......"]),
        );
        assert_filter(
            &buf,
            CellFilter::AnyOf(filters.clone()),
            &Buffer::with_lines(["......", "XXXXXX", ".XXXX.", "......"]),
        );
        assert_filter(
            &buf,
            CellFilter::NoneOf(filters),
            &Buffer::with_lines(["XXXXXX", "......", "X....X", "XXXXXX"]),
        );
    }

    #[test]
    fn test_ref_area_filter() {
        let empty = Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . . "]);
        let fx = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_symbol("X");
            }
        });

        let ref_rect = RefRect::new(Rect::new(2, 1, 4, 2));
        let mut buf = empty.clone();
        let filter = CellFilter::RefArea(ref_rect.clone());

        let area = *buf.area();
        buf.render_effect(
            &mut fx.clone().with_filter(filter),
            area,
            Duration::from_millis(16),
        );

        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". XXXX. ", ". XXXX. ", ". . . . ",])
        );

        // Test that changing the RefRect updates the filter area
        ref_rect.set(Rect::new(0, 0, 2, 2));
        let mut buf2 = empty;
        let filter2 = CellFilter::RefArea(ref_rect);
        buf2.render_effect(
            &mut fx.with_filter(filter2),
            area,
            Duration::from_millis(16),
        );

        assert_eq!(
            buf2,
            Buffer::with_lines(["XX. . . ", "XX. . . ", ". . . . ", ". . . . ",])
        );
    }

    #[test]
    fn test_ref_area_filter_equality() {
        let ref_rect1 = RefRect::new(Rect::new(0, 0, 10, 10));
        let ref_rect2 = RefRect::new(Rect::new(0, 0, 10, 10));
        let ref_rect3 = RefRect::new(Rect::new(5, 5, 10, 10));

        let filter1 = CellFilter::RefArea(ref_rect1);
        let filter2 = CellFilter::RefArea(ref_rect2);
        let filter3 = CellFilter::RefArea(ref_rect3.clone());

        assert_eq!(filter1, filter2);
        assert_ne!(filter1, filter3);

        // Test that changing one RefRect affects equality
        ref_rect3.set(Rect::new(0, 0, 10, 10));
        let filter4 = CellFilter::RefArea(ref_rect3);
        assert_eq!(filter1, filter4);
    }

    #[test]
    fn test_static_filter_analyzer() {
        use crate::cell_filter::analyzer::{FilterAnalyzer, FilterType};

        let static_text = CellFilter::Static(Box::new(CellFilter::Text));
        let static_area = CellFilter::Static(Box::new(CellFilter::Area(Rect::new(0, 0, 10, 10))));

        // Both should be analyzed as static regardless of wrapped filter type
        assert_eq!(static_text.analyze(), FilterType::Static);
        assert_eq!(static_area.analyze(), FilterType::Static);
    }

    #[test]
    fn test_static_vs_dynamic_filters() {
        use ratatui_core::style::{Color, Style};

        let mut buf = Buffer::filled(Rect::new(0, 0, 6, 4), Cell::new("."));
        buf.set_span(
            0,
            1,
            &ratatui_core::text::Span::from("......").style(Style::default().fg(Color::Red)),
            6,
        );
        buf.set_span(
            0,
            2,
            &ratatui_core::text::Span::from("......").style(Style::default().bg(Color::Blue)),
            6,
        );

        let fx = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_symbol("X");
                c.set_style(Style::reset());
            }
        });

        let test_filter = |dynamic: CellFilter, static_wrapped: CellFilter| {
            let mut buf1 = buf.clone();
            let mut buf2 = buf.clone();
            buf1.render_effect(
                &mut fx.clone().with_filter(dynamic),
                buf.area,
                Duration::from_millis(16),
            );
            buf2.render_effect(
                &mut fx.clone().with_filter(static_wrapped),
                buf.area,
                Duration::from_millis(16),
            );
            assert_eq!(buf1, buf2);
            buf1
        };

        // Test color filters
        let mut fg_result = test_filter(
            CellFilter::FgColor(Color::Red),
            CellFilter::Static(Box::new(CellFilter::FgColor(Color::Red))),
        );
        let mut bg_result = test_filter(
            CellFilter::BgColor(Color::Blue),
            CellFilter::Static(Box::new(CellFilter::BgColor(Color::Blue))),
        );

        // Clear remaining styles for clean comparison
        let mut clear_styles = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_style(Style::reset());
            }
        });
        clear_styles.process(Duration::from_millis(16), &mut fg_result, buf.area);
        clear_styles.process(Duration::from_millis(16), &mut bg_result, buf.area);

        assert_eq!(
            fg_result,
            Buffer::with_lines(["......", "XXXXXX", "......", "......"])
        );
        assert_eq!(
            bg_result,
            Buffer::with_lines(["......", "......", "XXXXXX", "......"])
        );

        // Test text filter
        let mut text_buf = Buffer::filled(Rect::new(0, 0, 8, 3), Cell::new(" "));
        text_buf.set_span(0, 0, &ratatui_core::text::Span::from("Hello123"), 8);
        text_buf.set_span(0, 1, &ratatui_core::text::Span::from("────────"), 8);
        text_buf.set_span(0, 2, &ratatui_core::text::Span::from("Test!()"), 7);

        let text_fx = effect_fn((), 1, |_, _, cells| {
            for (_, c) in cells {
                c.set_symbol("X");
            }
        });

        let mut text1 = text_buf.clone();
        let mut text2 = text_buf.clone();
        text1.render_effect(
            &mut text_fx.clone().with_filter(CellFilter::Text),
            text_buf.area,
            Duration::from_millis(16),
        );
        text2.render_effect(
            &mut text_fx.with_filter(CellFilter::Static(Box::new(CellFilter::Text))),
            text_buf.area,
            Duration::from_millis(16),
        );

        assert_eq!(text1, text2);
        assert_eq!(
            text1,
            Buffer::with_lines(["XXXXXXXX", "────────", "XXXXXXXX"])
        );
    }
}
