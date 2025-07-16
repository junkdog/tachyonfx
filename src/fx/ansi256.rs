use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Color,
};

use crate::{default_shader_impl, shader::Shader, CellFilter, Duration, LruCache};

#[derive(Clone, Default, Debug)]
pub struct Ansi256 {
    area: Option<Rect>,
}

impl Shader for Ansi256 {
    default_shader_impl!(area, clone);

    fn name(&self) -> &'static str {
        "term256_colors"
    }

    fn process(&mut self, _duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let mut fg_cache: LruCache<Color, Color, 4> = LruCache::default();
        let mut bg_cache: LruCache<Color, Color, 4> = LruCache::default();

        let safe_area = area.intersection(buf.area);
        for y in area.top()..safe_area.bottom() {
            for x in area.left()..safe_area.right() {
                let cell = buf.cell_mut(Position::new(x, y))?;
                let fg = fg_cache.memoize(&cell.fg, |c| {
                    #[allow(deprecated)]
                    crate::color_ext::AsIndexedColor::as_indexed_color(c)
                });
                let bg = bg_cache.memoize(&cell.bg, |c| {
                    #[allow(deprecated)]
                    crate::color_ext::AsIndexedColor::as_indexed_color(c)
                });

                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        None
    }

    fn done(&self) -> bool {
        false
    }

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
    #[allow(deprecated)]
    fn to_dsl() {
        use crate::shader::Shader;

        let dsl = fx::term256_colors().to_dsl().unwrap().to_string();
        assert_eq!(dsl, "fx::term256_colors()");
    }
}
