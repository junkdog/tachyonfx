use alloc::boxed::Box;
use core::fmt::Debug;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::{
    default_shader_impl,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    CellFilter, Duration, EffectTimer, FilterProcessor, Shader,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Evolve {
    symbol_set: EvolveSymbolSet,
    pattern: AnyPattern,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    style: Option<Style>,
}

impl Evolve {
    pub(crate) fn new(symbols: EvolveSymbolSet, lifetime: EffectTimer) -> Self {
        Self {
            symbol_set: symbols,
            timer: lifetime,
            pattern: AnyPattern::Identity,
            area: None,
            cell_filter: None,
            style: None,
        }
    }

    /// Sets a custom style for the evolve effect symbols
    pub(crate) fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Shader for Evolve {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        "evolve"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let symbols = self.symbol_set;
        let style = self.style;

        let mut pattern = self.pattern.for_frame(alpha, area);
        self.cell_iter(buf, area)
            .for_each_cell(|pos, cell| {
                let cell_alpha = pattern.map_alpha(pos);
                let symbol = symbols.get_symbol(cell_alpha);
                cell.set_char(symbol);

                // Apply custom style if provided
                if let Some(style) = style {
                    cell.set_style(style);
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

#[derive(Clone, Debug, Copy, Default)]
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

    const fn len(&self) -> usize {
        self.symbols().len()
    }

    fn get_symbol(&self, alpha: f32) -> char {
        let len = self.len();
        let idx = (alpha * (len as f32 - 1.0)).round() as usize;
        self.symbols()[idx.min(len - 1)]
    }
}
