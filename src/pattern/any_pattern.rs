use ratatui::layout::{Position, Rect};

use crate::{
    fx::sliding_window_alpha::SlidingWindowAlpha,
    pattern::{
        CheckerboardPattern, CoalescePattern, DiagonalPattern, DissolvePattern, InstancedPattern,
        Pattern, PreparedPattern, RadialPattern, SweepPattern,
    },
    simple_rng::SimpleRng,
};

/// An enum that can hold any concrete pattern type.
/// This allows shaders to store patterns without knowing their concrete types at compile
/// time.
#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub enum AnyPattern {
    #[default]
    Identity, // Returns global alpha unchanged - allows single code path for all effects
    Radial(RadialPattern),
    Diagonal(DiagonalPattern),
    Checkerboard(CheckerboardPattern),
    Sweep(SweepPattern),
    Coalesce(CoalescePattern),
    Dissolve(DissolvePattern),
}

/// Context enum that holds the appropriate pattern frame state for each pattern type
pub enum AnyPatternContext {
    Identity(f32), // Just stores the global alpha
    Radial(PreparedPattern<(f32, Rect), RadialPattern>),
    Diagonal(PreparedPattern<(f32, Rect), DiagonalPattern>),
    Checkerboard(PreparedPattern<(f32, Rect), CheckerboardPattern>),
    Sweep(PreparedPattern<SlidingWindowAlpha, SweepPattern>),
    Coalesce(PreparedPattern<(f32, SimpleRng), CoalescePattern>),
    Dissolve(PreparedPattern<(f32, SimpleRng), DissolvePattern>),
}

impl Pattern for AnyPattern {
    type Context = AnyPatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        use AnyPatternContext as APC;
        let context = match self {
            AnyPattern::Identity => APC::Identity(alpha),
            AnyPattern::Radial(pattern) => APC::Radial(pattern.for_frame(alpha, area)),
            AnyPattern::Diagonal(pattern) => APC::Diagonal(pattern.for_frame(alpha, area)),
            AnyPattern::Checkerboard(pattern) => APC::Checkerboard(pattern.for_frame(alpha, area)),
            AnyPattern::Sweep(pattern) => APC::Sweep(pattern.for_frame(alpha, area)),
            AnyPattern::Coalesce(pattern) => APC::Coalesce(pattern.for_frame(alpha, area)),
            AnyPattern::Dissolve(pattern) => APC::Dissolve(pattern.for_frame(alpha, area)),
        };

        PreparedPattern { pattern: self, context }
    }
}

impl InstancedPattern for PreparedPattern<AnyPatternContext, AnyPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        match &mut self.context {
            AnyPatternContext::Identity(alpha) => *alpha, // Just return the global alpha unchanged
            AnyPatternContext::Radial(frame) => frame.map_alpha(pos),
            AnyPatternContext::Diagonal(frame) => frame.map_alpha(pos),
            AnyPatternContext::Checkerboard(frame) => frame.map_alpha(pos),
            AnyPatternContext::Sweep(frame) => frame.map_alpha(pos),
            AnyPatternContext::Coalesce(frame) => frame.map_alpha(pos),
            AnyPatternContext::Dissolve(frame) => frame.map_alpha(pos),
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

impl From<DissolvePattern> for AnyPattern {
    fn from(pattern: DissolvePattern) -> Self {
        AnyPattern::Dissolve(pattern)
    }
}
