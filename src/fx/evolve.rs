use alloc::boxed::Box;
use core::fmt::Debug;

use ratatui_core::{buffer::Buffer, layout::Rect, style::Style};

use crate::{
    default_shader_impl,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    CellFilter, Duration, EffectTimer, FilterProcessor, Shader,
};

#[derive(Clone, Debug, Default)]
pub(super) struct Evolve {
    symbol_set: EvolveSymbolSet,
    pattern: AnyPattern,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    style: Option<Style>,
    mode: EvolveMode,
}

/// Controls how evolve effects interact with underlying buffer content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum EvolveMode {
    /// Always updates buffer with evolved symbols
    #[default]
    Full,
    /// Reveals underlying content at alpha=1.0 (evolves into existing content)
    Into,
    /// Reveals underlying content at alpha=0.0 (evolves from existing content)
    From,
}

impl Evolve {
    pub(super) fn new(symbols: impl Into<EvolveSymbolConfig>, lifetime: EffectTimer) -> Self {
        let (symbols, style) = match symbols.into() {
            EvolveSymbolConfig::Plain(symbols) => (symbols, None),
            EvolveSymbolConfig::Styled(symbols, style) => (symbols, Some(style)),
        };

        Self {
            symbol_set: symbols,
            timer: lifetime,
            pattern: AnyPattern::Identity,
            style,
            ..Self::default()
        }
    }

    pub(super) fn with_mode(mut self, mode: EvolveMode) -> Self {
        self.mode = mode;
        self
    }
}

impl Shader for Evolve {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        match self.mode {
            EvolveMode::Into => "evolve_into",
            EvolveMode::From => "evolve_from",
            EvolveMode::Full => "evolve",
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let symbols = self.symbol_set;
        let style = self.style;
        let mode = self.mode;

        let mut pattern = self.pattern.clone().for_frame(alpha, area);
        self.cell_iter(buf, area)
            .for_each_cell(|pos, cell| {
                let cell_alpha = pattern.map_alpha(pos);

                let should_draw = !((mode == EvolveMode::From && cell_alpha == 0.0)
                    || (mode == EvolveMode::Into && cell_alpha == 1.0));

                if should_draw {
                    let symbol = symbols.get_symbol(cell_alpha);
                    cell.set_char(symbol);

                    if let Some(style) = style {
                        cell.set_style(style);
                    }
                }
            });
    }

    fn set_pattern(&mut self, pattern: AnyPattern) {
        self.pattern = pattern;
    }
}

pub(crate) enum EvolveSymbolConfig {
    Plain(EvolveSymbolSet),
    Styled(EvolveSymbolSet, Style),
}

impl From<EvolveSymbolSet> for EvolveSymbolConfig {
    fn from(value: EvolveSymbolSet) -> Self {
        EvolveSymbolConfig::Plain(value)
    }
}

impl From<(EvolveSymbolSet, Style)> for EvolveSymbolConfig {
    fn from(value: (EvolveSymbolSet, Style)) -> Self {
        EvolveSymbolConfig::Styled(value.0, value.1)
    }
}

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub enum EvolveSymbolSet {
    BlocksHorizontal,
    BlocksVertical,
    CircleFill,
    #[default]
    Circles,
    Quadrants,
    Shaded,
    Squares,
}

impl EvolveSymbolSet {
    const fn symbols(&self) -> &[char] {
        match self {
            EvolveSymbolSet::Circles => &[' ', '·', '•', '◉', '●'],
            EvolveSymbolSet::BlocksHorizontal => &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'],
            EvolveSymbolSet::BlocksVertical => &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'],
            EvolveSymbolSet::CircleFill => &[' ', '◌', '◎', '◍', '●'],
            EvolveSymbolSet::Quadrants => {
                &[' ', '▖', '▘', '▗', '▝', '▚', '▞', '▙', '▛', '▜', '▟', '█']
            },
            EvolveSymbolSet::Shaded => &[' ', '░', '▒', '▓', '█'],
            EvolveSymbolSet::Squares => &[' ', '·', '▫', '▪', '◼', '█'],
        }
    }

    const fn len(self) -> usize {
        self.symbols().len()
    }

    fn get_symbol(self, alpha: f32) -> char {
        let len = self.len();
        let idx = crate::math::round(alpha * (len as f32 - 1.0)) as usize;
        self.symbols()[idx.min(len - 1)]
    }
}
