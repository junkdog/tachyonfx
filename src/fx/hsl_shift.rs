use bon::{builder, Builder};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::{CellFilter, Duration};
use crate::{ColorMapper, HslConvertable, Interpolatable};
use crate::dsl::{DslError, DslFormat, EffectExpression};

#[derive(Builder, Clone, Default, Debug)]
pub struct HslShift {
    #[builder(into)]
    timer: EffectTimer,
    hsl_mod_fg: Option<[f32; 3]>,
    hsl_mod_bg: Option<[f32; 3]>,
    area: Option<Rect>,
    #[builder(default)]
    cell_filter: CellFilter,
}

impl Shader for HslShift {
    fn name(&self) -> &'static str {
        "hsl_shift"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();

        let cell_iter = self.cell_iter(buf, area);
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        let hsl_lerp = |c: Color, hsl: [f32; 3]| -> Color {
            let (h, s, l) = c.to_hsl_f32();

            let (h, s, l) = (
                (h + 0.0.lerp(&hsl[0], alpha)) % 360.0,
                (s + 0.0.lerp(&hsl[1], alpha)).clamp(0.0, 100.0),
                (l + 0.0.lerp(&hsl[2], alpha)).clamp(0.0, 100.0),
            );

            HslConvertable::from_hsl_f32(h, s, l)
        };

        for (_, cell) in cell_iter {
            if let Some(hsl_mod) = self.hsl_mod_fg {
                let fg = fg_mapper.map(cell.fg, alpha, |c| hsl_lerp(c, hsl_mod));
                cell.set_fg(fg);
            }
            if let Some(hsl_mod) = self.hsl_mod_bg {
                let bg = bg_mapper.map(cell.bg, alpha, |c| hsl_lerp(c, hsl_mod));
                cell.set_bg(bg);
            }

        }
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { self.area }
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

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        let hsl_mod_fg = self.hsl_mod_fg
            .map(|hsl| format!("Some([{}, {}, {}])", hsl[0], hsl[1], hsl[2]))
            .unwrap_or("None".to_string());

        let hsl_mod_bg = self.hsl_mod_bg
            .map(|hsl| format!("Some([{}, {}, {}])", hsl[0], hsl[1], hsl[2]))
            .unwrap_or("None".to_string());

        EffectExpression::parse(&format!("{}({hsl_mod_fg}, {hsl_mod_bg}, {})",
            self.name(),
            self.timer.dsl_format(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use crate::{fx, Effect};
    use crate::dsl::{EffectDsl, EffectExpression};
    use crate::Interpolation::Linear;

    #[test]
    fn hsl_shift() {
        let input =   "fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear))";
        let expected = fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear));
        let result = compile_effect(input);
        assert_eq!(format!("{result:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_hsl_shift_fg() {
        let input =   "fx::hsl_shift_fg([1.0, 2.0, 3.0], (1000, Linear))";
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
                Some([
                    1.0,
                    2.0,
                    3.0
                ]),
                Some([
                    1.0,
                    2.0,
                    3.0
                ]),
                EffectTimer::new(
                    Duration::from_millis(1000),
                    Interpolation::Linear
                )
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