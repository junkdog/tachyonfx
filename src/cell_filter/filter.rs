use std::fmt;

use ratatui::{
    buffer::Cell,
    layout,
    layout::{Margin, Position, Rect},
    prelude::Color,
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
#[derive(Clone, Default)]
pub enum CellFilter {
    /// Selects every cell
    #[default]
    All,
    /// Selects cells within the specified area
    Area(Rect),
    /// Selects cells within the area defined by a RefRect
    RefArea(RefRect),
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
    where
        F: Fn(&Cell) -> bool + ThreadSafetyMarker + 'static,
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
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        fn to_hex(c: &Color) -> String {
            let (r, g, b) = c.to_rgb();
            format!("#{r:02x}{g:02x}{b:02x}")
        }

        fn format_margin(m: &Margin) -> String {
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
            CellFilter::FgColor(color) => format!("fg({})", to_hex(color)),
            CellFilter::BgColor(color) => format!("bg({})", to_hex(color)),
            CellFilter::Inner(m) => format!("inner({})", format_margin(m)),
            CellFilter::Outer(m) => format!("outer({})", format_margin(m)),
            CellFilter::Text => "text".to_string(),
            CellFilter::AllOf(filters) => format!("all_of({})", to_string(filters)),
            CellFilter::AnyOf(filters) => format!("any_of({})", to_string(filters)),
            CellFilter::NoneOf(filters) => format!("none_of({})", to_string(filters)),
            CellFilter::Not(filter) => format!("!{}", filter.to_string()),
            CellFilter::Layout(_, idx) => format!("layout({idx})"),
            CellFilter::PositionFn(_) => "position_fn".to_string(),
            CellFilter::EvalCell(_) => "eval_cell".to_string(),
        }
    }

    pub fn predicate(&self, area: Rect) -> CellPredicate<'_> {
        CellPredicate::new(area, self)
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
            CellFilter::AllOf(filters) => f.debug_tuple("AllOf").field(filters).finish(),
            CellFilter::AnyOf(filters) => f.debug_tuple("AnyOf").field(filters).finish(),
            CellFilter::NoneOf(filters) => f.debug_tuple("NoneOf").field(filters).finish(),
            CellFilter::Not(filter) => f.debug_tuple("Not").field(filter).finish(),
            CellFilter::Layout(layout, idx) => {
                write!(f, "Layout({layout:?}, {idx})")
            },
            CellFilter::PositionFn(_) => write!(f, "PositionFn(<function>)"),
            CellFilter::EvalCell(_) => write!(f, "EvalCell(<function>)"),
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
    use layout::Layout;
    use ratatui::{buffer::Buffer, style::Style, text::Span};

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

        let filter =
            CellFilter::Layout(Layout::horizontal(&[] as &[ratatui::layout::Constraint]), 0);
        assert_eq!(filter.to_string(), "layout(0)");

        let filter = CellFilter::PositionFn(ref_count(|_| true));
        assert_eq!(filter.to_string(), "position_fn");

        let filter = CellFilter::EvalCell(ref_count(|_| true));
        assert_eq!(filter.to_string(), "eval_cell");

        let ref_rect = RefRect::new(Rect::new(5, 10, 20, 30));
        let filter = CellFilter::RefArea(ref_rect);
        assert_eq!(filter.to_string(), "ref_area(20x30+5+10)");
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

        let mut buf = empty.clone();
        let filter = CellFilter::Not(Box::new(CellFilter::Area(Rect::new(0, 0, 8, 2))));
        buf.render_effect(
            &mut fx.clone().with_filter(filter),
            area,
            Duration::from_millis(16),
        );

        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", "XXXXXXXX", "XXXXXXXX",])
        );
    }

    #[test]
    fn test_all_any_and_none_of() {
        fn assert_filter(buf: &Buffer, filter: CellFilter, expected: Buffer) {
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

            assert_eq!(b, expected);
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
            Buffer::with_lines(["......", ".XXXX.", "......", "......"]),
        );
        assert_filter(
            &buf,
            CellFilter::AnyOf(filters.clone()),
            Buffer::with_lines(["......", "XXXXXX", ".XXXX.", "......"]),
        );
        assert_filter(
            &buf,
            CellFilter::NoneOf(filters.clone()),
            Buffer::with_lines(["XXXXXX", "......", "X....X", "XXXXXX"]),
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
        let mut buf2 = empty.clone();
        let filter2 = CellFilter::RefArea(ref_rect.clone());
        buf2.render_effect(
            &mut fx.clone().with_filter(filter2),
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

        let filter1 = CellFilter::RefArea(ref_rect1.clone());
        let filter2 = CellFilter::RefArea(ref_rect2);
        let filter3 = CellFilter::RefArea(ref_rect3.clone());

        assert_eq!(filter1, filter2);
        assert_ne!(filter1, filter3);

        // Test that changing one RefRect affects equality
        ref_rect3.set(Rect::new(0, 0, 10, 10));
        let filter4 = CellFilter::RefArea(ref_rect3);
        assert_eq!(filter1, filter4);
    }
}
