use std::cell::RefCell;
use std::rc::Rc;
use ratatui::buffer::Cell;
use ratatui::layout;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::prelude::Color;
use crate::color_ext::ToRgbComponents;

/// A filter mode enables effects to operate on specific cells.
#[derive(Clone, Default)]
pub enum CellFilter {
    /// Selects every cell
    #[default]
    All,
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
    PositionFn(Rc<RefCell<dyn Fn(Position) -> bool>>),
}

impl CellFilter {
    pub fn position_fn<F>(f: F) -> Self
        where F: Fn(Position) -> bool + 'static
    {
        CellFilter::PositionFn(Rc::new(RefCell::new(f)))
    }

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
            CellFilter::All => "all".to_string(),
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
        }
    }
}

pub struct CellSelector {
    inner_area: Rect,
    strategy: CellFilter,
}

impl CellSelector {
    fn new(area: Rect, strategy: CellFilter) -> Self {
        let inner_area = Self::resolve_area(area, &strategy);

        Self { inner_area, strategy }
    }

    fn resolve_area(area: Rect, mode: &CellFilter) -> Rect {
        match mode {
            CellFilter::All                  => area,
            CellFilter::Inner(margin)        => area.inner(*margin),
            CellFilter::Outer(margin)        => area.inner(*margin),
            CellFilter::Text                 => area,
            CellFilter::AllOf(_)             => area,
            CellFilter::AnyOf(_)             => area,
            CellFilter::NoneOf(_)            => area,
            CellFilter::Not(m)               => Self::resolve_area(area, m.as_ref()),
            CellFilter::FgColor(_)           => area,
            CellFilter::BgColor(_)           => area,
            CellFilter::Layout(layout, idx)  => layout.split(area)[*idx as usize],
            CellFilter::PositionFn(_)        => area,
        }
    }

    pub fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        let mode = &self.strategy;

        self.valid_position(pos, mode)
            && self.is_valid_cell(cell, mode)
    }

    fn valid_position(&self, pos: Position, mode: &CellFilter) -> bool {
        match mode {
            CellFilter::All           => self.inner_area.contains(pos),
            CellFilter::Layout(_, _)  => self.inner_area.contains(pos),
            CellFilter::Inner(_)      => self.inner_area.contains(pos),
            CellFilter::Outer(_)      => !self.inner_area.contains(pos),
            CellFilter::Text          => self.inner_area.contains(pos),
            CellFilter::AllOf(s)      => s.iter()
                .all(|mode| mode.selector(self.inner_area).valid_position(pos, mode)),
            CellFilter::AnyOf(s)      => s.iter()
                .any(|mode| mode.selector(self.inner_area).valid_position(pos, mode)),
            CellFilter::NoneOf(s)     => s.iter()
                .all(|mode| !mode.selector(self.inner_area).valid_position(pos, mode)),
            CellFilter::Not(m)        => self.valid_position(pos, m.as_ref()),
            CellFilter::FgColor(_)    => self.inner_area.contains(pos),
            CellFilter::BgColor(_)    => self.inner_area.contains(pos),
            CellFilter::PositionFn(f) => f.borrow()(pos),
        }
    }

    fn is_valid_cell(&self, cell: &Cell, mode: &CellFilter) -> bool {
        match mode {
            CellFilter::Text => {
                if cell.symbol().len() == 1 {
                    let ch = cell.symbol().chars().next().unwrap();
                    ch.is_alphabetic() || ch.is_numeric() || ch == ' ' || "?!.,:;".contains(ch)
                } else {
                    false
                }
            },

            CellFilter::AllOf(s) => {
                s.iter()
                    .all(|s| s.selector(self.inner_area).is_valid_cell(cell, s))
            },

            CellFilter::FgColor(color) => cell.fg == *color,
            CellFilter::BgColor(color) => cell.bg == *color,

            CellFilter::Not(m) => !self.is_valid_cell(cell, m.as_ref()),

            _ => true,
        }
    }
}

impl CellFilter {
    pub fn selector(&self, area: Rect) -> CellSelector {
        CellSelector::new(area, self.clone())
    }
}

#[cfg(test)]
mod tests {
    use layout::Layout;
    use super::*;

    #[test]
    fn test_cell_filter_to_string() {
        let filter = CellFilter::FgColor(Color::Red);
        assert_eq!(filter.to_string(), "fg(#ff0000)");

        let filter = CellFilter::BgColor(Color::Green);
        assert_eq!(filter.to_string(), "bg(#00ff00)");

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
        assert_eq!(filter.to_string(), "all_of(fg(#ff0000), bg(#00ff00))");

        let filter = CellFilter::AnyOf(vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::BgColor(Color::Green),
        ]);
        assert_eq!(filter.to_string(), "any_of(fg(#ff0000), bg(#00ff00))");

        let filter = CellFilter::NoneOf(vec![
            CellFilter::FgColor(Color::Red),
            CellFilter::BgColor(Color::Green),
        ]);
        assert_eq!(filter.to_string(), "none_of(fg(#ff0000), bg(#00ff00))");

        let filter = CellFilter::Not(Box::new(CellFilter::FgColor(Color::Red)));
        assert_eq!(filter.to_string(), "!fg(#ff0000)");

        let filter = CellFilter::Layout(Layout::horizontal(&[]), 0);
        assert_eq!(filter.to_string(), "layout(0)");

        let filter = CellFilter::PositionFn(Rc::new(RefCell::new(|_| true)));
        assert_eq!(filter.to_string(), "position_fn");
    }
}