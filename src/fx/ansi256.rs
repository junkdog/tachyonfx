use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};

use crate::Duration;
use crate::color_ext::AsIndexedColor;
use crate::color_mapper::ColorMapper;
use crate::CellFilter;
use crate::dsl::{DslError, EffectExpression};
use crate::shader::Shader;

#[derive(Clone, Default, Debug)]
pub struct Ansi256 {
    area: Option<Rect>,
}

impl Shader for Ansi256 {
    fn name(&self) -> &'static str {
        "term256_colors"
    }

    fn process(
        &mut self,
        _duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        let safe_area = area.intersection(buf.area);
        for y in area.top()..safe_area.bottom() {
            for x in area.left()..safe_area.right() {
                let cell = buf.cell_mut(Position::new(x, y))?;
                let fg = fg_mapper.map(cell.fg, 0.0, |c| c.as_indexed_color());
                let bg = bg_mapper.map(cell.bg, 0.0, |c| c.as_indexed_color());

                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        None
    }

    fn done(&self) -> bool { false }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {}

    fn reset(&mut self) {}

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        EffectExpression::parse("fx::term256_colors()")
    }
}

#[cfg(test)]
mod tests {
    use crate::fx;

    #[test]
    fn to_dsl() {
        use crate::shader::Shader;

        let dsl = fx::term256_colors().to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::term256_colors()");
    }
}