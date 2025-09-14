use core::fmt::Debug;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::{
    default_shader_impl,
    pattern::{InstancedPattern, Pattern, PatternForFrame},
    CellFilter, Duration, EffectTimer, FilterProcessor, Shader,
};

#[derive(Clone, Debug)]
pub(crate) struct Evolve<P>
where
    P: Pattern + Copy + Send + 'static,
    PatternForFrame<P::Context, P>: InstancedPattern,
{
    symbol_set: EvolveSymbolSet,
    pattern: P,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    style: Option<Style>,
}

impl<P: Pattern + Copy + Send + 'static> Evolve<P>
where
    PatternForFrame<P::Context, P>: InstancedPattern,
{
    pub(crate) fn new(symbols: EvolveSymbolSet, pattern: P, lifetime: EffectTimer) -> Self {
        Self {
            symbol_set: symbols,
            timer: lifetime,
            pattern,
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

impl<P: Pattern + Debug + Copy + Send + 'static> Shader for Evolve<P>
where
    PatternForFrame<P::Context, P>: InstancedPattern,
{
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
}

#[derive(Clone, Debug, Copy)]
pub enum EvolveSymbolSet {
    BlocksHorizontal,
    BlocksVertical,
    CircleFill,
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
            EvolveSymbolSet::CircleFill => &[' ', '◌', '◍', '◎', '●'],
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
