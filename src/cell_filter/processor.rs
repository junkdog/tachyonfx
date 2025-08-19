use std::ops::{BitAnd, BitOr, Not};

use bitvec::{bits, bitvec, vec::BitVec};
use ratatui::{
    buffer::Cell,
    layout::{Position, Rect},
};

use crate::{
    cell_filter::{analyzer::FilterType, FilterAnalyzer},
    CellFilter, CellPredicate, RefRect,
};
// struct FilterProcessor {
//     area: Option<Rect>,
// }
//
// enum Op {
//     AllOf,
//     NoneOf,
//     AnyOf,
//     Not,
// }
//
// enum FilterAction {
//     Dynamic(CellFilter),
//     Hybrid(Op, Vec<CellFilter>),
//     Static(StaticFilterProcessor),
// }

#[derive(Debug, Clone)]
pub(crate) enum FilterProcessor {
    Static(StaticFilterProcessor),
    Dynamic(CellFilter, Rect),
}

impl FilterProcessor {
    pub fn selector(&self, area: Rect) -> CellPredicate<'_> {
        match self {
            FilterProcessor::Static(processor) => processor.filter.selector(area),
            FilterProcessor::Dynamic(filter, _) => filter.selector(area),
        }
    }

    pub(crate) fn new(filter: CellFilter) -> Self {
        let area = Rect::default();
        match filter.analyze() {
            FilterType::Static => FilterProcessor::Static(StaticFilterProcessor::new(filter, area)),
            FilterType::Dynamic => FilterProcessor::Dynamic(filter, area),
        }
    }

    pub(crate) fn update(&mut self, area: Rect) {
        if let FilterProcessor::Static(processor) = self {
            processor.update(area);
        }
    }

    pub(crate) fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        match self {
            FilterProcessor::Static(processor) => processor.is_valid(pos),
            FilterProcessor::Dynamic(filter, area) => filter.selector(*area).is_valid(pos, cell),
        }
    }

    pub(crate) fn filter_ref(&self) -> &CellFilter {
        match self {
            FilterProcessor::Static(processor) => &processor.filter,
            FilterProcessor::Dynamic(filter, _) => filter,
        }
    }
}

#[derive(Debug, Clone)]
struct StaticFilterProcessor {
    filter: CellFilter,
    cell_indices: BitVec,
    last_active_area: Rect,
    ref_rects: Vec<(Rect, RefRect)>,
}

impl From<CellFilter> for FilterProcessor {
    fn from(value: CellFilter) -> Self {
        FilterProcessor::new(value)
    }
}

impl StaticFilterProcessor {
    fn new(filter: CellFilter, area: Rect) -> Self {
        let cell_indices = calculate_cell_indices(area, &filter);
        let ref_rects = find_ref_rects(&filter);
        Self {
            filter,
            cell_indices,
            last_active_area: area,
            ref_rects,
        }
    }

    fn update(&mut self, area: Rect) {
        if !self.check_resize(area) {
            self.cell_indices = calculate_cell_indices(area, &self.filter);
            self.ref_rects = find_ref_rects(&self.filter);
            self.last_active_area = area;
        }
    }

    fn is_valid_index(&self, index: usize) -> bool {
        if index >= self.cell_indices.len() {
            return false; // Out of bounds
        }

        self.cell_indices[index]
    }

    pub(crate) fn is_valid(&self, pos: Position) -> bool {
        self.is_valid_index((pos.y * self.last_active_area.width + pos.x) as usize)
    }

    fn check_resize(&mut self, area: Rect) -> bool {
        for (rect, ref_rect) in &self.ref_rects {
            if rect != &area || ref_rect.get() != area {
                return false; // Area has changed, need to recalculate
            }
        }

        if self.last_active_area != area {
            self.last_active_area = area;
            return false; // Area has changed, need to recalculate
        }

        true
    }
}

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
        _ => {}, // Other filters do not have ref rects
    }

    ref_rects
}

fn calculate_cell_indices(area: Rect, filter: &CellFilter) -> BitVec {
    let size = area.width * area.height;
    let mut cell_indices = BitVec::new();
    cell_indices.resize(size as _, false);

    let mut activate_area = |r: Rect, v: bool| {
        for y in r.top()..r.bottom() {
            for x in r.left()..r.right() {
                let index = (y * area.width) + x;
                cell_indices.set(index as usize, v);
            }
        }
    };

    match &filter {
        CellFilter::All => activate_area(area, true),
        CellFilter::Area(r) => activate_area(*r, true),
        CellFilter::RefArea(r) => activate_area(r.get(), true), // todo: handle dynamic udpates
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
                .map(|f| calculate_cell_indices(area, f))
                .reduce(|acc, i| acc.bitand(i));

            if let Some(indices) = all_of {
                cell_indices = indices;
            }
        },
        CellFilter::AnyOf(filters) => {
            let any_of = filters
                .iter()
                .map(|f| calculate_cell_indices(area, f))
                .reduce(|acc, i| acc.bitor(i));

            if let Some(indices) = any_of {
                cell_indices = indices;
            }
        },
        CellFilter::NoneOf(filters) => {
            let none_of = filters
                .iter()
                .map(|f| calculate_cell_indices(area, f))
                .reduce(|acc, i| acc.bitor(i))
                .map(|indices| indices.not());

            if let Some(indices) = none_of {
                cell_indices = indices;
            }
        },
        CellFilter::Not(filter) => {
            let indices = calculate_cell_indices(area, filter);
            cell_indices = indices.not();
        },

        // _ => BitVec::from_elem(area.width * area.height, false),
        _ => todo!("tbd: {:?}", filter),
    };

    cell_indices
}
