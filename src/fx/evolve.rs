use core::fmt::Debug;
use crate::CellFilter;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{default_shader_impl, Duration, EffectTimer, FilterProcessor, Shader};
use crate::pattern::Pattern;

#[derive(Clone, Debug)]
pub(crate) struct Evolve<P>
    where P: Pattern + Copy + Send + 'static,
{
    symbol_set: EvolveSymbolSet,
    pattern: P,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
}

impl<P: Pattern + Copy + Send + 'static> Evolve<P> {
    pub(crate) fn new(
        symbols: EvolveSymbolSet,
        pattern: P,
        lifetime: EffectTimer
    ) -> Self {
        Self {
            symbol_set: symbols,
            timer: lifetime,
            pattern,
            area: None,
            cell_filter: None,
        }
    }
}

impl<P: Pattern + Debug + Copy + Send + 'static> Shader for Evolve<P> {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        "evolve"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let symbols = self.symbol_set;
        let mut pattern = self.pattern;

        self.cell_iter(buf, area).for_each_cell(|pos, cell| {
            let cell_alpha = pattern.transform_alpha(alpha, (pos.x, pos.y), area);
            let symbol = symbols.get_symbol(cell_alpha);
            cell.set_char(symbol);
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
            EvolveSymbolSet::Quadrants => &[' ', '▖', '▘', '▗', '▝', '▚', '▞', '▙', '▛', '▜', '▟', '█'],
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

