use alloc::boxed::Box;

#[cfg(feature = "dsl")]
use compact_str;
use ratatui_core::{buffer::Buffer, layout::Rect, style::Style};

use crate::{
    default_shader_impl,
    fx::{
        expand::ExpandDirection::{Horizontal, Vertical},
        stretch::Stretch,
    },
    CellFilter, Duration, EffectTimer, Motion, Shader,
};

/// Direction for bidirectional expansion effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExpandDirection {
    /// Expands horizontally from center (left and right simultaneously)
    Horizontal,
    /// Expands vertically from center (up and down simultaneously)
    Vertical,
}

/// A shader that applies bidirectional expansion effects using two opposing stretch
/// shaders.
///
/// Creates expansion animations that grow outward from the center in both directions
/// simultaneously, either horizontally or vertically.
#[derive(Clone, Debug)]
pub(super) struct Expand {
    direction: ExpandDirection,
    stretch_a: Stretch,
    stretch_b: Stretch,
    /// The area within which the effect is applied.
    area: Option<Rect>,
}

impl Expand {
    /// Creates a new expand effect with the specified direction, style, and timing.
    pub(super) fn new(direction: ExpandDirection, style: Style, timer: EffectTimer) -> Self {
        use ExpandDirection::*;

        Self {
            direction,
            stretch_a: Stretch::builder()
                .style(style)
                .timer(timer)
                .direction(match direction {
                    Horizontal => Motion::RightToLeft,
                    Vertical => Motion::DownToUp,
                })
                .build(),
            stretch_b: Stretch::builder()
                .style(style)
                .timer(timer)
                .direction(match direction {
                    Horizontal => Motion::LeftToRight,
                    Vertical => Motion::UpToDown,
                })
                .build(),
            area: None,
        }
    }
}

impl Shader for Expand {
    default_shader_impl!(area, clone);

    fn name(&self) -> &'static str {
        "Expand"
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.stretch_a.cell_filter()
    }

    fn done(&self) -> bool {
        self.stretch_a.done()
    }

    fn filter(&mut self, filter: CellFilter) {
        self.stretch_a.filter(filter.clone());
        self.stretch_b.filter(filter);
    }

    fn reset(&mut self) {
        self.stretch_a.reset();
        self.stretch_b.reset();
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        // safe area
        let area = self.area.unwrap_or(area).intersection(buf.area);

        let (area_a, area_b) = match self.direction {
            Horizontal => {
                let area_a = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width / 2,
                    height: area.height,
                };
                let area_b = Rect {
                    x: area_a.right(),
                    y: area.y,
                    width: area.width - area_a.width,
                    height: area.height,
                };
                (area_a, area_b)
            },
            Vertical => {
                let area_a = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: area.height / 2,
                };
                let area_b = Rect {
                    x: area.x,
                    y: area_a.bottom(),
                    width: area.width,
                    height: area.height - area_a.height,
                };
                (area_a, area_b)
            },
        };

        self.stretch_a.process(duration, buf, area_a);
        self.stretch_b.process(duration, buf, area_b)
    }

    fn reverse(&mut self) {
        self.stretch_a.reverse();
        self.stretch_b.reverse();
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        let direction_str = match self.direction {
            Horizontal => "ExpandDirection::Horizontal",
            Vertical => "ExpandDirection::Vertical",
        };

        EffectExpression::parse(&compact_str::format_compact!(
            "fx::expand({}, {}, {})",
            direction_str,
            self.stretch_a.style.dsl_format(),
            self.stretch_a.timer.dsl_format(),
        ))
    }
}
