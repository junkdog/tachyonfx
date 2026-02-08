use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

use crate::{
    cell_filter::FilterProcessor,
    default_shader_impl,
    effect_timer::EffectTimer,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    shader::Shader,
    CellFilter, Duration,
};

#[derive(Clone, Debug)]
pub(super) struct Paint {
    fg: Option<Color>,
    bg: Option<Color>,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    pattern: AnyPattern,
}

impl Paint {
    pub fn new(fg: Option<Color>, bg: Option<Color>, timer: EffectTimer) -> Self {
        if fg.is_none() && bg.is_none() {
            panic!("At least one of fg or bg must be Some");
        }

        Self {
            fg,
            bg,
            timer,
            area: None,
            cell_filter: None,
            pattern: AnyPattern::default(),
        }
    }
}

impl Shader for Paint {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        match (self.fg, self.bg) {
            (Some(_), Some(_)) => "paint",
            (Some(_), None) => "paint_fg",
            (None, Some(_)) => "paint_bg",
            (None, None) => unimplemented!("not possible"),
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let global_alpha = self.timer.alpha();
        let fg = self.fg;
        let bg = self.bg;

        let is_identity_pattern = matches!(self.pattern, AnyPattern::Identity);
        let mut pattern = self.pattern.clone().for_frame(global_alpha, area);
        let cell_iter = self.cell_iter(buf, area);

        cell_iter.for_each_cell(move |pos, cell| {
            if is_identity_pattern || pattern.map_alpha(pos) > 0.0 {
                if let Some(fg) = fg.as_ref() {
                    cell.set_fg(*fg);
                }

                if let Some(bg) = bg.as_ref() {
                    cell.set_bg(*bg);
                }
            }
        });
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let s = match (self.fg, self.bg) {
            (Some(fg), Some(bg)) => {
                format!(
                    "fx::paint({}, {}, {})",
                    fg.dsl_format(),
                    bg.dsl_format(),
                    self.timer.dsl_format(),
                )
            },
            (Some(fg), None) => {
                format!(
                    "fx::paint_fg({}, {})",
                    fg.dsl_format(),
                    self.timer.dsl_format(),
                )
            },
            (None, Some(bg)) => {
                format!(
                    "fx::paint_bg({}, {})",
                    bg.dsl_format(),
                    self.timer.dsl_format(),
                )
            },
            (None, None) => unreachable!("At least one of fg or bg must be Some"),
        };
        crate::dsl::EffectExpression::parse(&s)
    }

    fn set_pattern(&mut self, pattern: AnyPattern) {
        self.pattern = pattern;
    }
}
