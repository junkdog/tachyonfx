mod checkerboard;
mod coalesce;
mod diagonal;
mod radial;
mod sweep;
mod util;

use ratatui::layout::{Position, Rect};

pub use self::{
    checkerboard::CheckerboardPattern,
    coalesce::CoalescePattern,
    diagonal::{DiagonalDirection, DiagonalPattern},
    radial::RadialPattern,
    sweep::SweepPattern,
};

pub(crate) trait Pattern {
    type Context;

    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized;
}

pub(crate) trait InstancedPattern {
    fn map_alpha(&mut self, pos: Position) -> f32;
}

pub(crate) struct PatternForFrame<S, P: Pattern> {
    pattern: P,
    context: S,
}
