use ratatui::layout::{Position, Rect};

use crate::pattern::{
    CheckerboardPattern, CoalescePattern, DiagonalPattern, InstancedPattern, Pattern,
    PatternForFrame, RadialPattern, SweepPattern,
};

/// An enum that can hold any concrete pattern type.
/// This allows shaders to store patterns without knowing their concrete types at compile
/// time.
#[derive(Clone, Debug, Copy)]
pub enum AnyPattern {
    Identity, // Returns global alpha unchanged - allows single code path for all effects
    Radial(RadialPattern),
    Diagonal(DiagonalPattern),
    Checkerboard(CheckerboardPattern),
    Sweep(SweepPattern),
    Coalesce(CoalescePattern),
}

impl Default for AnyPattern {
    fn default() -> Self {
        AnyPattern::Identity
    }
}

impl Pattern for AnyPattern {
    type Context = (f32, Rect);

    fn for_frame(self, alpha: f32, area: Rect) -> PatternForFrame<Self::Context, Self>
    where
        Self: Sized,
    {
        PatternForFrame { pattern: self, context: (alpha, area) }
    }
}

impl InstancedPattern for PatternForFrame<(f32, Rect), AnyPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let (alpha, area) = self.context;
        match &mut self.pattern {
            AnyPattern::Identity => alpha, // Just return the global alpha unchanged
            AnyPattern::Radial(pattern) => {
                let mut radial_frame = pattern.for_frame(alpha, area);
                radial_frame.map_alpha(pos)
            },
            AnyPattern::Diagonal(pattern) => {
                let mut diagonal_frame = pattern.for_frame(alpha, area);
                diagonal_frame.map_alpha(pos)
            },
            AnyPattern::Checkerboard(pattern) => {
                let mut checkerboard_frame = pattern.for_frame(alpha, area);
                checkerboard_frame.map_alpha(pos)
            },
            AnyPattern::Sweep(pattern) => {
                let mut sweep_frame = pattern.for_frame(alpha, area);
                sweep_frame.map_alpha(pos)
            },
            AnyPattern::Coalesce(pattern) => {
                let mut coalesce_frame = pattern.for_frame(alpha, area);
                coalesce_frame.map_alpha(pos)
            },
        }
    }
}

// Implement From for each concrete pattern type
impl From<RadialPattern> for AnyPattern {
    fn from(pattern: RadialPattern) -> Self {
        AnyPattern::Radial(pattern)
    }
}

impl From<DiagonalPattern> for AnyPattern {
    fn from(pattern: DiagonalPattern) -> Self {
        AnyPattern::Diagonal(pattern)
    }
}

impl From<CheckerboardPattern> for AnyPattern {
    fn from(pattern: CheckerboardPattern) -> Self {
        AnyPattern::Checkerboard(pattern)
    }
}

impl From<SweepPattern> for AnyPattern {
    fn from(pattern: SweepPattern) -> Self {
        AnyPattern::Sweep(pattern)
    }
}

impl From<CoalescePattern> for AnyPattern {
    fn from(pattern: CoalescePattern) -> Self {
        AnyPattern::Coalesce(pattern)
    }
}
