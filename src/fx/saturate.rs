use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    cell_filter::FilterProcessor,
    default_shader_impl,
    effect_timer::EffectTimer,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    shader::Shader,
    CellFilter, ColorCache, ColorSpace, Duration,
};

#[derive(Clone, Debug)]
pub(super) struct Saturate {
    fg: Option<f32>,
    bg: Option<f32>,
    timer: EffectTimer,
    area: Option<Rect>,
    color_space: ColorSpace,
    cell_filter: Option<FilterProcessor>,
    pattern: AnyPattern,
}

impl Saturate {
    pub fn new(fg: Option<f32>, bg: Option<f32>, timer: EffectTimer) -> Self {
        if fg.is_none() && bg.is_none() {
            panic!("At least one of fg or bg must be Some");
        }

        Self {
            fg,
            bg,
            timer,
            area: None,
            color_space: ColorSpace::Rgb,
            cell_filter: None,
            pattern: AnyPattern::default(),
        }
    }
}

impl Shader for Saturate {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        match (self.fg, self.bg) {
            (Some(_), Some(_)) => "saturate",
            (Some(_), None) => "saturate_fg",
            (None, Some(_)) => "saturate_bg",
            (None, None) => unimplemented!("not possible"),
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let global_alpha = self.timer.alpha();
        let fg_saturate = self.fg;
        let bg_saturate = self.bg;
        let color_space = self.color_space;

        let mut pattern = self.pattern.clone().for_frame(global_alpha, area);
        let cell_iter = self.cell_iter(buf, area);

        let mut color_cache: ColorCache<u32, 8> = ColorCache::new();

        cell_iter.for_each_cell(move |pos, cell| {
            let alpha = pattern.map_alpha(pos);

            if let Some(factor) = fg_saturate {
                let modified_alpha = 1.0 + factor * alpha;
                let color = color_cache.memoize_fg(cell.fg, modified_alpha.to_bits(), |c| {
                    color_space.saturate(c, modified_alpha)
                });
                cell.set_fg(color);
            }

            if let Some(factor) = bg_saturate.as_ref().copied() {
                let modified_alpha = 1.0 + factor * alpha;
                let color = color_cache.memoize_bg(cell.bg, modified_alpha.to_bits(), |c| {
                    color_space.saturate(c, modified_alpha)
                });
                cell.set_bg(color);
            }
        });
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let fmt_opt = |v: Option<f32>| match v {
            Some(f) => format!("Some({})", f.dsl_format()),
            None => "None".into(),
        };

        let s = match (self.fg, self.bg) {
            (Some(fg), Some(bg)) => {
                format!(
                    "fx::saturate({}, {}, {})",
                    fmt_opt(Some(fg)),
                    fmt_opt(Some(bg)),
                    self.timer.dsl_format(),
                )
            },
            (Some(fg), None) => {
                format!(
                    "fx::saturate_fg({}, {})",
                    fg.dsl_format(),
                    self.timer.dsl_format(),
                )
            },
            (None, Some(bg)) => {
                format!(
                    "fx::saturate(None, {}, {})",
                    fmt_opt(Some(bg)),
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
