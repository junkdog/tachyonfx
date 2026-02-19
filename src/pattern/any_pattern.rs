use ratatui_core::layout::{Position, Rect};

use crate::{
    fx::sliding_window_alpha::SlidingWindowAlpha,
    pattern::{
        blend::{BlendPattern, BlendPatternContext},
        combined::{CombinedPattern, CombinedPatternContext},
        diagonal::DiagonalContext,
        diamond::{DiamondContext, DiamondPattern},
        inverted::{InvertedPattern, InvertedPatternContext},
        radial::RadialContext,
        spiral::{SpiralContext, SpiralPattern},
        wave::WavePatternContext,
        CheckerboardPattern, CoalescePattern, DiagonalPattern, DissolvePattern, InstancedPattern,
        Pattern, PreparedPattern, RadialPattern, SweepPattern, WavePattern,
    },
    simple_rng::SimpleRng,
};

/// An enum that can hold any concrete pattern type.
/// This allows shaders to store patterns without knowing their concrete types at compile
/// time.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AnyPattern {
    #[default]
    Identity, // Returns global alpha unchanged - allows single code path for all effects
    Radial(RadialPattern),
    Diamond(DiamondPattern),
    Diagonal(DiagonalPattern),
    Checkerboard(CheckerboardPattern),
    Sweep(SweepPattern),
    Coalesce(CoalescePattern),
    Dissolve(DissolvePattern),
    Blend(BlendPattern),
    Combined(CombinedPattern),
    Inverted(InvertedPattern),
    Wave(WavePattern),
    Spiral(SpiralPattern),
}

/// Context enum that holds the appropriate pattern frame state for each pattern type
pub enum AnyPatternContext {
    Identity(f32), // Just stores the global alpha
    Radial(PreparedPattern<RadialContext, RadialPattern>),
    Diamond(PreparedPattern<DiamondContext, DiamondPattern>),
    Diagonal(PreparedPattern<DiagonalContext, DiagonalPattern>),
    Checkerboard(PreparedPattern<(f32, Rect), CheckerboardPattern>),
    Sweep(PreparedPattern<SlidingWindowAlpha, SweepPattern>),
    Coalesce(PreparedPattern<(f32, SimpleRng), CoalescePattern>),
    Dissolve(PreparedPattern<(f32, SimpleRng), DissolvePattern>),
    Blend(PreparedPattern<BlendPatternContext, BlendPattern>),
    Combined(PreparedPattern<CombinedPatternContext, CombinedPattern>),
    Inverted(PreparedPattern<InvertedPatternContext, InvertedPattern>),
    Wave(PreparedPattern<WavePatternContext, WavePattern>),
    Spiral(PreparedPattern<SpiralContext, SpiralPattern>),
}

impl Pattern for AnyPattern {
    type Context = AnyPatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        use AnyPatternContext as APC;
        let pattern = self.clone();
        let context = match self {
            AnyPattern::Identity => APC::Identity(alpha),
            AnyPattern::Radial(p) => APC::Radial(p.for_frame(alpha, area)),
            AnyPattern::Diamond(p) => APC::Diamond(p.for_frame(alpha, area)),
            AnyPattern::Diagonal(p) => APC::Diagonal(p.for_frame(alpha, area)),
            AnyPattern::Checkerboard(p) => APC::Checkerboard(p.for_frame(alpha, area)),
            AnyPattern::Sweep(p) => APC::Sweep(p.for_frame(alpha, area)),
            AnyPattern::Coalesce(p) => APC::Coalesce(p.for_frame(alpha, area)),
            AnyPattern::Dissolve(p) => APC::Dissolve(p.for_frame(alpha, area)),
            AnyPattern::Blend(p) => APC::Blend(p.for_frame(alpha, area)),
            AnyPattern::Combined(p) => APC::Combined(p.for_frame(alpha, area)),
            AnyPattern::Inverted(p) => APC::Inverted(p.for_frame(alpha, area)),
            AnyPattern::Wave(p) => APC::Wave(p.for_frame(alpha, area)),
            AnyPattern::Spiral(p) => APC::Spiral(p.for_frame(alpha, area)),
        };

        PreparedPattern { pattern, context }
    }
}

impl InstancedPattern for PreparedPattern<AnyPatternContext, AnyPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        match &mut self.context {
            AnyPatternContext::Identity(alpha) => *alpha, // Just return the global alpha unchanged
            AnyPatternContext::Radial(frame) => frame.map_alpha(pos),
            AnyPatternContext::Diamond(frame) => frame.map_alpha(pos),
            AnyPatternContext::Diagonal(frame) => frame.map_alpha(pos),
            AnyPatternContext::Checkerboard(frame) => frame.map_alpha(pos),
            AnyPatternContext::Sweep(frame) => frame.map_alpha(pos),
            AnyPatternContext::Coalesce(frame) => frame.map_alpha(pos),
            AnyPatternContext::Dissolve(frame) => frame.map_alpha(pos),
            AnyPatternContext::Blend(frame) => frame.map_alpha(pos),
            AnyPatternContext::Combined(frame) => frame.map_alpha(pos),
            AnyPatternContext::Inverted(frame) => frame.map_alpha(pos),
            AnyPatternContext::Wave(frame) => frame.map_alpha(pos),
            AnyPatternContext::Spiral(frame) => frame.map_alpha(pos),
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

impl From<WavePattern> for AnyPattern {
    fn from(pattern: WavePattern) -> Self {
        AnyPattern::Wave(pattern)
    }
}

impl From<BlendPattern> for AnyPattern {
    fn from(pattern: BlendPattern) -> Self {
        AnyPattern::Blend(pattern)
    }
}

impl From<DiamondPattern> for AnyPattern {
    fn from(pattern: DiamondPattern) -> Self {
        AnyPattern::Diamond(pattern)
    }
}

impl From<SpiralPattern> for AnyPattern {
    fn from(pattern: SpiralPattern) -> Self {
        AnyPattern::Spiral(pattern)
    }
}

impl From<CombinedPattern> for AnyPattern {
    fn from(pattern: CombinedPattern) -> Self {
        AnyPattern::Combined(pattern)
    }
}

impl From<InvertedPattern> for AnyPattern {
    fn from(pattern: InvertedPattern) -> Self {
        AnyPattern::Inverted(pattern)
    }
}
