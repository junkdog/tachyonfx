use std::ops::{BitAnd, BitOr, Not};

use bitvec::vec::BitVec;
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
pub enum FilterProcessor {
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
            FilterType::Static => FilterProcessor::Static(StaticFilterProcessor::new(filter)),
            FilterType::Dynamic => FilterProcessor::Dynamic(filter, area),
        }
    }

    pub(crate) fn update(&mut self, area: Rect) {
        match self {
            FilterProcessor::Static(processor) => processor.update(area),
            FilterProcessor::Dynamic(_, a) => *a = area,
        }
    }

    pub(crate) fn validator(&self) -> CellValidator<'_> {
        match self {
            FilterProcessor::Static(processor) => CellValidator::Static(processor),
            FilterProcessor::Dynamic(filter, area) => {
                CellValidator::Dynamic(filter.selector(*area))
            },
        }
    }

    pub(crate) fn filter_ref(&self) -> &CellFilter {
        match self {
            FilterProcessor::Static(processor) => &processor.filter,
            FilterProcessor::Dynamic(filter, _) => filter,
        }
    }
}

pub(crate) enum CellValidator<'a> {
    Static(&'a StaticFilterProcessor),
    Dynamic(CellPredicate<'a>),
}

impl CellValidator<'_> {
    pub(crate) fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        match self {
            CellValidator::Static(processor) => processor.is_valid(pos),
            CellValidator::Dynamic(predicate) => predicate.is_valid(pos, cell),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticFilterProcessor {
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
    fn new(filter: CellFilter) -> Self {
        let ref_rects = find_ref_rects(&filter);

        Self {
            filter,
            cell_indices: BitVec::new(),
            last_active_area: Rect::default(),
            ref_rects,
        }
    }

    fn is_valid(&self, pos: Position) -> bool {
        let row_offset = pos.y as usize * self.last_active_area.width as usize;
        self.is_valid_index(row_offset + pos.x as usize)
    }

    fn update(&mut self, area: Rect) {
        if self.requires_resize(area) {
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

    fn requires_resize(&mut self, area: Rect) -> bool {
        for (rect, ref_rect) in &self.ref_rects {
            if rect != &area || ref_rect.get() != area {
                return true; // Area has changed, need to recalculate
            }
        }

        if self.last_active_area != area {
            self.last_active_area = area;
            return true; // Area has changed, need to recalculate
        }

        false
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
        CellFilter::RefArea(r) => activate_area(r.get(), true),
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

        _ => unimplemented!("only static filters (hybrid not yet impl): {:?}", filter),
    };

    cell_indices
}
