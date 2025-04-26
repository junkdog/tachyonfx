use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::simple_rng::SimpleRng;
use crate::{default_shader_impl, CellFilter, Duration};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

#[derive(Clone, Debug, Default)]
pub struct Dissolve {
    timer: EffectTimer,
    dissolved_style: Option<Style>,
    area: Option<Rect>,
    cell_filter: Option<CellFilter>,
    lcg: SimpleRng,
}

impl Dissolve {
    pub fn new(
        lifetime: EffectTimer,
    ) -> Self {
        Self {
            timer: lifetime,
            ..Self::default()
        }
    }

    pub fn with_style(
        style: Style,
        lifetime: EffectTimer,
    ) -> Self {
        Self {
            dissolved_style: Some(style),
            timer: lifetime,
            ..Self::default()
        }
    }
}

impl Shader for Dissolve {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        match (self.dissolved_style, self.timer.is_reversed()) {
            (Some(_), true)  => "coalesce_from",
            (Some(_), false) => "dissolve_to",
            (None, true)     => "coalesce",
            (None, false)    => "dissolve",
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let cell_iter = self.cell_iter(buf, area);
        let mut lcg = self.lcg;

        let dissolved_cells = cell_iter
            .filter(|_| alpha > lcg.gen_f32());

        if let Some(style) = self.dissolved_style {
            dissolved_cells.for_each(|(_, c)| {
                c.set_char(' ');
                c.set_style(style);
            });
        } else {
            dissolved_cells.for_each(|(_, c)| {
                c.set_char(' ');
            });
        }
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        if self.dissolved_style.is_none() {
            EffectExpression::parse(&format!(
                "fx::{}({})",
                self.name(),
                self.timer.dsl_format(),
            ))
        } else {
            let style = self.dissolved_style.as_ref().unwrap().dsl_format();
            EffectExpression::parse(&format!(
                "fx::{}({}, {})",
                self.name(),
                style,
                self.timer.dsl_format(),
            ))
        }
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use crate::Interpolation::SineOut;
    use crate::{fx, EffectTimer, Shader};
    use indoc::indoc;
    use ratatui::style::Style;

    #[test]
    fn dsl_format_dissolve() {
        assert_eq!(
            fx::dissolve(1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::dissolve(EffectTimer::from_ms(1000, Interpolation::Linear))"
            }
        );
    }

    #[test]
    fn dsl_format_coalesce() {
        assert_eq!(
            fx::coalesce(1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::coalesce(EffectTimer::from_ms(1000, Interpolation::Linear))"
            }
        );
    }

    #[test]
    fn dsl_format_dissolve_to() {
        let dissolve = fx::dissolve_to(Style::default(), EffectTimer::from_ms(100, SineOut)).to_dsl().unwrap();
        assert_eq!(
            dissolve.to_string(),
            indoc! {
                "fx::dissolve_to(Style::new(), EffectTimer::from_ms(100, Interpolation::SineOut))"
            }
        );
    }

    #[test]
    fn dsl_format_coalesce_from() {
        assert_eq!(
            fx::coalesce_from(Style::default(), 1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::coalesce_from(Style::new(), EffectTimer::from_ms(1000, Interpolation::Linear))"
            }
        );
    }
}