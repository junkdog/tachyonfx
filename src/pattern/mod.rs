mod any_pattern;
mod checkerboard;
mod coalesce;
mod diagonal;
mod instanced_pattern;
mod radial;
mod sweep;
mod util;

use ratatui::layout::Rect;

pub(crate) use self::instanced_pattern::InstancedPattern;
pub use self::{
    any_pattern::AnyPattern,
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

pub(crate) struct PatternForFrame<S, P: Pattern> {
    pattern: P,
    context: S,
}
