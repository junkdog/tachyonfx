use crate::color_ext::AsIndexedColor;
use crate::shader::Shader;
use crate::{default_shader_impl, CellFilter};
use crate::{Duration, LruCache};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

#[derive(Clone, Default, Debug)]
pub struct Ansi256 {
    area: Option<Rect>,
}

impl Shader for Ansi256 {
    default_shader_impl!(area, clone);

    fn name(&self) -> &'static str {
        "term256_colors"
    }

    fn process(
        &mut self,
        _duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let mut fg_cache: LruCache<Color, Color, 4> = LruCache::default();
        let mut bg_cache: LruCache<Color, Color, 4> = LruCache::default();

        let safe_area = area.intersection(buf.area);
        for y in area.top()..safe_area.bottom() {
            for x in area.left()..safe_area.right() {
                let cell = buf.cell_mut(Position::new(x, y))?;
                let fg = fg_cache.memoize(&cell.fg, |c| c.as_indexed_color());
                let bg = bg_cache.memoize(&cell.bg, |c| c.as_indexed_color());

                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        None
    }

    fn done(&self) -> bool { false }

    fn filter(&mut self, _strategy: CellFilter) {}

    fn reset(&mut self) {}

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse("fx::term256_colors()")
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use crate::fx;

    #[test]
    fn to_dsl() {
        use crate::shader::Shader;

        let dsl = fx::term256_colors().to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::term256_colors()");
    }
}