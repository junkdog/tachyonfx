use bitvec::vec::BitVec;
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

struct StaticFilterProcessor {
    filter: CellFilter,
    cell_indices: BitVec,
}
