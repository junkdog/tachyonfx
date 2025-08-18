use std::ops::{BitAnd, BitOr, Not};

use bitvec::{bits, bitvec, vec::BitVec};
use ratatui::layout::Rect;

use crate::CellFilter;

struct FilterProcessor {
    area: Option<Rect>,
}

enum Op {
    AllOf,
    NoneOf,
    AnyOf,
    Not,
}

enum FilterAction {
    Dynamic(CellFilter),
    Hybrid(Op, Vec<CellFilter>),
    Static(StaticFilterProcessor),
}

#[derive(Debug)]
struct StaticFilterProcessor {
    filter: CellFilter,
    cell_indices: BitVec,
    ref_area: Option<Rect>,
}

impl StaticFilterProcessor {
    fn new(filter: CellFilter, area: Rect) -> Self {
        let cell_indices = calculate_cell_indices(area, &filter);
        Self { filter, cell_indices, ref_area: None }
    }
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

        // _ => BitVec::from_elem(area.width * area.height, false),
        _ => todo!("tbd"),
    };

    cell_indices
}
