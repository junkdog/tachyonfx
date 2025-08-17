use bon::{builder, Builder};
use ratatui::{buffer::Buffer, layout::Rect, prelude::Color};

use crate::{
    default_shader_impl, effect_timer::EffectTimer, shader::Shader, CellFilter, ColorCache,
    ColorSpace, Duration,
};

#[derive(Builder, Clone, Debug)]
pub struct FadeColors {
    fg: Option<Color>,
    bg: Option<Color>,
    #[builder(into)]
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<CellFilter>,
    color_space: ColorSpace,
}

impl Shader for FadeColors {
    default_shader_impl!(area, timer, filter, color_space, clone);

    fn name(&self) -> &'static str {
        if self.timer.is_reversed() {
            "fade_from"
        } else {
            "fade_to"
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();

        let cell_iter = self.cell_iter(buf, area);
        let mut color_cache: ColorCache<8> = ColorCache::new();

        cell_iter.for_each(|(_, cell)| {
            if let Some(fg) = self.fg.as_ref() {
                let color = color_cache.memoize_fg(cell.fg, *fg, alpha, |_| {
                    self.color_space.lerp(&cell.fg, fg, alpha)
                });
                cell.set_fg(color);
            }

            if let Some(bg) = self.bg.as_ref() {
                let color = color_cache.memoize_bg(cell.bg, *bg, alpha, |_| {
                    self.color_space.lerp(&cell.bg, bg, alpha)
                });
                cell.set_bg(color);
            }
        });
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let s = if self.bg.is_some() {
            format!(
                "fx::{}({}, {}, {})",
                self.name(),
                self.fg.unwrap().dsl_format(),
                self.bg.unwrap().dsl_format(),
                self.timer.dsl_format(),
            )
        } else {
            format!(
                "fx::{}_fg({}, {})",
                self.name(),
                self.fg.unwrap().dsl_format(),
                self.timer.dsl_format()
            )
        };
        crate::dsl::EffectExpression::parse(&s)
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;
    use ratatui::style::Color;

    use crate::{effect_timer::EffectTimer, fx, shader::Shader, Interpolation::QuadOut};

    #[test]
    fn to_dsl_fade_to_fg() {
        let dsl = fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
        });
    }

    #[test]
    fn to_dsl_fade_to() {
        let dsl = fx::fade_to(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        )
        .to_dsl()
        .unwrap()
        .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_to(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
        });
    }

    #[test]
    fn to_dsl_fade_from_fg() {
        let dsl = fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
        });
    }

    #[test]
    fn to_dsl_fade_from() {
        let dsl = fx::fade_from(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        )
        .to_dsl()
        .unwrap()
        .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_from(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
        });
    }
}
