use bon::{builder, Builder};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Color;

use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::{CellFilter, Duration, Interpolatable, LruCache};

#[derive(Builder, Clone, Debug)]
pub struct FadeColors {
    fg: Option<Color>,
    bg: Option<Color>,
    #[builder(into)]
    timer: EffectTimer,
    area: Option<Rect>,
    #[builder(default)]
    cell_filter: CellFilter,
}

impl Shader for FadeColors {
    fn name(&self) -> &'static str {
        if self.timer.is_reversed() { "fade_from" } else { "fade_to" }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();

        let cell_iter = self.cell_iter(buf, area);
        let mut fg_cache: LruCache<Color, Color, 8> = LruCache::new();
        let mut bg_cache: LruCache<Color, Color, 8> = LruCache::new();

        cell_iter.for_each(|(_, cell)| {
            if let Some(fg) = self.fg.as_ref() {
                let color = fg_cache.memoize(&cell.fg, |c| c.lerp(fg, alpha));
                cell.set_fg(color);
            }

            if let Some(bg) = self.bg.as_ref() {
                let color = bg_cache.memoize(&cell.bg, |c| c.lerp(bg, alpha));
                cell.set_bg(color);
            }
        });
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
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
    use crate::effect_timer::EffectTimer;
    use crate::fx;
    use crate::shader::Shader;
    use crate::Interpolation::QuadOut;
    use indoc::indoc;
    use ratatui::style::Color;

    #[test]
    fn to_dsl_fade_to_fg() {
        let dsl = fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(
            dsl,
            indoc! {
                "fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
            }
        );
    }

    #[test]
    fn to_dsl_fade_to() {
        let dsl = fx::fade_to(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        ).to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(
            dsl,
            indoc! {
                "fx::fade_to(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
            }
        );
    }

    #[test]
    fn to_dsl_fade_from_fg() {
        let dsl = fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(
            dsl,
            indoc! {
                "fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
            }
        );
    }

    #[test]
    fn to_dsl_fade_from() {
        let dsl = fx::fade_from(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        ).to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(
            dsl,
            indoc! {
                "fx::fade_from(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
            }
        );
    }
}