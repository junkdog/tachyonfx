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
pub(super) struct Lighten {
    /// Lightness adjustment for foreground: -1.0 (black) to 1.0 (white).
    fg: Option<f32>,
    /// Lightness adjustment for background: -1.0 (black) to 1.0 (white).
    bg: Option<f32>,
    timer: EffectTimer,
    area: Option<Rect>,
    color_space: ColorSpace,
    cell_filter: Option<FilterProcessor>,
    pattern: AnyPattern,
}

impl Lighten {
    pub fn new(fg: Option<f32>, bg: Option<f32>, timer: EffectTimer) -> Self {
        if fg.is_none() && bg.is_none() {
            panic!("At least one of fg or bg must be Some");
        }

        Self {
            fg: fg.map(|v| v.clamp(-1.0, 1.0)),
            bg: bg.map(|v| v.clamp(-1.0, 1.0)),
            timer,
            area: None,
            color_space: ColorSpace::Rgb,
            cell_filter: None,
            pattern: AnyPattern::default(),
        }
    }

    fn is_darken(&self) -> bool {
        self.fg.unwrap_or(0.0) < 0.0 || self.bg.unwrap_or(0.0) < 0.0
    }
}

impl Shader for Lighten {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        let darken = self.is_darken();
        match (self.fg, self.bg) {
            (Some(_), Some(_)) if darken => "darken",
            (Some(_), None) if darken => "darken_fg",
            (None, Some(_)) if darken => "darken_bg",
            (Some(_), Some(_)) => "lighten",
            (Some(_), None) => "lighten_fg",
            (None, Some(_)) => "lighten_bg",
            (None, None) => unimplemented!("not possible"),
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let global_alpha = self.timer.alpha();
        let fg_amount = self.fg;
        let bg_amount = self.bg;
        let color_space = self.color_space;

        let mut pattern = self.pattern.clone().for_frame(global_alpha, area);
        let cell_iter = self.cell_iter(buf, area);

        let mut color_cache: ColorCache<u32, 8> = ColorCache::new();

        cell_iter.for_each_cell(move |pos, cell| {
            let alpha = pattern.map_alpha(pos);

            if let Some(amount) = fg_amount {
                let scaled = amount * alpha;
                let color = color_cache.memoize_fg(cell.fg, scaled.to_bits(), |c| {
                    color_space.lighten(c, scaled)
                });
                cell.set_fg(color);
            }

            if let Some(amount) = bg_amount {
                let scaled = amount * alpha;
                let color = color_cache.memoize_bg(cell.bg, scaled.to_bits(), |c| {
                    color_space.lighten(c, scaled)
                });
                cell.set_bg(color);
            }
        });
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let fmt_opt = |v: Option<f32>| match v {
            Some(f) => format!("Some({})", f.abs().dsl_format()),
            None => "None".into(),
        };

        let darken = self.is_darken();
        let (name, name_fg) =
            if darken { ("darken", "darken_fg") } else { ("lighten", "lighten_fg") };

        let s = match (self.fg, self.bg) {
            (Some(fg), Some(bg)) => {
                format!(
                    "fx::{}({}, {}, {})",
                    name,
                    fmt_opt(Some(fg)),
                    fmt_opt(Some(bg)),
                    self.timer.dsl_format(),
                )
            },
            (Some(fg), None) => {
                format!(
                    "fx::{}({}, {})",
                    name_fg,
                    fg.abs().dsl_format(),
                    self.timer.dsl_format(),
                )
            },
            (None, Some(bg)) => {
                format!(
                    "fx::{}(None, {}, {})",
                    name,
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
