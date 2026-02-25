use alloc::boxed::Box;

use bon::Builder;
use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

use crate::{
    cell_filter::FilterProcessor,
    color_space::color_from_hsl,
    color_to_hsl, default_shader_impl,
    effect_timer::EffectTimer,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    shader::Shader,
    CellFilter, ColorCache, Duration, Interpolatable,
};

#[derive(Builder, Clone, Default, Debug)]
pub(super) struct HslShift {
    #[builder(into)]
    timer: EffectTimer,
    hsl_mod_fg: Option<[f32; 3]>,
    hsl_mod_bg: Option<[f32; 3]>,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    #[builder(default)]
    pattern: AnyPattern,
}

impl Shader for HslShift {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        "hsl_shift"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let global_alpha = self.timer.alpha();

        let hsl_lerp = |c: &Color, hsl: [f32; 3], alpha: f32| -> Color {
            let (h, s, l) = color_to_hsl(c);

            let (h, s, l) = (
                ((h + 0.0.lerp(&hsl[0], alpha)) % 360.0 + 360.0) % 360.0,
                (s + 0.0.lerp(&hsl[1], alpha)).clamp(0.0, 100.0),
                (l + 0.0.lerp(&hsl[2], alpha)).clamp(0.0, 100.0),
            );

            color_from_hsl(h, s, l)
        };

        let hsl_mod_fg = self.hsl_mod_fg;
        let hsl_mod_bg = self.hsl_mod_bg;

        let mut pattern = self.pattern.clone().for_frame(global_alpha, area);

        let cell_iter = self.cell_iter(buf, area);
        let mut color_cache: ColorCache<u32, 8> = ColorCache::new();

        cell_iter.for_each_cell(|pos, cell| {
            if let Some(hsl_mod) = hsl_mod_fg {
                let alpha = pattern.map_alpha(pos);
                let alpha_bits = u32::from_le_bytes(alpha.to_le_bytes());
                let fg =
                    color_cache.memoize_fg(cell.fg, alpha_bits, |c| hsl_lerp(c, hsl_mod, alpha));
                cell.set_fg(fg);
            }
            if let Some(hsl_mod) = hsl_mod_bg {
                let alpha = pattern.map_alpha(pos);
                let alpha_bits = u32::from_le_bytes(alpha.to_le_bytes());
                let bg =
                    color_cache.memoize_bg(cell.bg, alpha_bits, |c| hsl_lerp(c, hsl_mod, alpha));
                cell.set_bg(bg);
            }
        });
    }

    fn set_pattern(&mut self, pattern: AnyPattern) {
        self.pattern = pattern;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        let hsl_mod_fg = self.hsl_mod_fg.map_or("None".to_string(), |hsl| {
            format!("Some([{}, {}, {}])", hsl[0], hsl[1], hsl[2])
        });

        let hsl_mod_bg = self.hsl_mod_bg.map_or("None".to_string(), |hsl| {
            format!("Some([{}, {}, {}])", hsl[0], hsl[1], hsl[2])
        });

        EffectExpression::parse(&format!(
            "{}({hsl_mod_fg}, {hsl_mod_bg}, {})",
            self.name(),
            self.timer.dsl_format(),
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;

    use crate::{
        dsl::{EffectDsl, EffectExpression},
        fx, Effect,
        Interpolation::Linear,
    };

    #[test]
    fn hsl_shift() {
        let input = "fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear))";
        let expected = fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear));
        let result = compile_effect(input);
        assert_eq!(format!("{result:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_hsl_shift_fg() {
        let input = "fx::hsl_shift_fg([1.0, 2.0, 3.0], (1000, Linear))";
        let expected = fx::hsl_shift_fg([1.0, 2.0, 3.0], (1000, Linear));
        let result = compile_effect(input);
        assert_eq!(format!("{result:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_expr_to_dsl() {
        let input = "fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear))";
        let result = EffectExpression::parse(input).unwrap();
        assert_eq!(format!("{result}"), indoc! {
            "fx::hsl_shift(
                Some([1.0, 2.0, 3.0]),
                Some([1.0, 2.0, 3.0]),
                (1000, Interpolation::Linear)
            )"
        });
    }

    fn compile_effect(input: &str) -> Effect {
        EffectDsl::new()
            .compiler()
            .compile(input)
            .unwrap()
    }
}
