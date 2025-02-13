use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::simple_rng::SimpleRng;
use crate::{CellFilter, Duration};
use crate::dsl::{DslError, DslFormat, EffectExpression};

#[derive(Clone, Debug, Default)]
pub struct Dissolve {
    timer: EffectTimer,
    dissolved_style: Option<Style>,
    area: Option<Rect>,
    cell_filter: CellFilter,
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
        self.area = Some(area)
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        if self.dissolved_style.is_none() {
            EffectExpression::parse(&format!(
                "{}({})",
                self.name(),
                self.timer.dsl_format(),
            ))
        } else {
            let style = self.dissolved_style.as_ref().unwrap().dsl_format();
            EffectExpression::parse(&format!(
                "{}({}, {})",
                self.name(),
                style,
                self.timer.dsl_format(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use ratatui::style::Style;
    use crate::{fx, EffectTimer, Shader};
    use crate::Interpolation::SineOut;

    #[test]
    fn dsl_format_dissolve() {
        assert_eq!(
            fx::dissolve(1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::dissolve(EffectTimer::from_ms(
                     1000,
                     Linear
                 ))"
            }
        );
    }

    #[test]
    fn dsl_format_coalesce() {
        assert_eq!(
            fx::coalesce(1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::coalesce(EffectTimer::from_ms(
                     1000,
                     Linear
                 ))"
            }
        );
    }

    #[test]
    fn dsl_format_dissolve_to() {
        let dissolve = fx::dissolve_to(Style::default(), EffectTimer::from_ms(100, SineOut)).to_dsl().unwrap();
        assert_eq!(
            dissolve.to_string(),
            indoc! {
                "fx::dissolve_to(
                     Style::new(),
                     EffectTimer::from_ms(
                         100,
                         SineOut
                     )
                 )"
            }
        );
    }

    #[test]
    fn dsl_format_coalesce_from() {
        assert_eq!(
            fx::coalesce_from(Style::default(), 1000).to_dsl().unwrap().to_string(),
            indoc! {
                "fx::coalesce_from(
                     Style::new(),
                     EffectTimer::from_ms(
                         1000,
                         Linear
                     )
                 )"
            }
        );
    }
}