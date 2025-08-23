use bon::{builder, Builder};
use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::{
    cell_filter::FilterProcessor, default_shader_impl, CellFilter, Duration, EffectTimer,
    Interpolatable, Motion, Shader,
};

/// A shader that applies a stretching effect to terminal cells, expanding or shrinking
/// rectangular areas using block characters.
#[derive(Builder, Clone, Debug)]
pub(super) struct Stretch {
    /// The style applied to the stretched area.
    style: Style,
    /// The direction of the stretching effect.
    direction: Motion,
    /// The timer controlling the duration and progress of the effect.
    #[builder(into)]
    timer: EffectTimer,
    /// The area within which the effect is applied.
    area: Option<Rect>,
    /// The cell selection strategy used to filter cells.
    cell_filter: Option<FilterProcessor>,
}

impl Stretch {
    fn fill_area(&mut self, area: Rect, buf: &mut Buffer) {
        let style = self.style;
        self.cell_iter(buf, area)
            .for_each_cell(|_pos, cell| {
                cell.set_symbol(" ");
                cell.set_style(style);
            });
    }
}

impl Shader for Stretch {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        "stretch"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();

        // determine the effective + safe area to apply the effect
        let area = self.area.unwrap_or(area).intersection(buf.area);
        self.fill_area(area, buf);

        if alpha == 1.0 {
            return;
        }

        let bounds = StretchBounds::new(area, self.direction, alpha);
        let fractional = bounds.end % 1.0;
        // For reverse directions, invert the fractional part since stretch characters
        // show progress from left-to-right, but we're stretching in the opposite direction
        let adjusted_fractional = match self.direction {
            Motion::RightToLeft | Motion::DownToUp => 1.0 - fractional,
            _ => fractional,
        };
        let (symbol, style) = stretch_symbol(adjusted_fractional, self.direction, self.style);

        use Motion::*;
        match self.direction {
            LeftToRight => {
                let x = bounds.end as u16;
                for y in area.top()..area.bottom() {
                    buf[(x, y)].set_char(symbol).set_style(style);
                }
            },
            RightToLeft => {
                let x = bounds.end as u16;
                for y in area.top()..area.bottom() {
                    buf[(x, y)].set_char(symbol).set_style(style);
                }
            },
            UpToDown => {
                let y = bounds.end as u16;
                for x in area.left()..area.right() {
                    buf[(x, y)].set_char(symbol).set_style(style);
                }
            },
            DownToUp => {
                let y = bounds.end as u16;
                for x in area.left()..area.right() {
                    buf[(x, y)].set_char(symbol).set_style(style);
                }
            },
        }
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        EffectExpression::parse(&format!(
            "fx::stretch({}, {}, {})",
            self.direction.dsl_format(),
            self.style.dsl_format(),
            self.timer.dsl_format()
        ))
    }
}

fn stretch_symbol_idx(alpha: f32) -> usize {
    let alpha = alpha.clamp(0.0, 1.0);
    (LAST_IDX as f32 * alpha).round() as usize
}

fn stretch_symbol(inside_cell_alpha: f32, motion: Motion, style: Style) -> (char, Style) {
    let char_idx = stretch_symbol_idx(inside_cell_alpha);

    use Motion::*;
    let symbol = match motion {
        LeftToRight => STRETCH_H[char_idx],
        RightToLeft => STRETCH_H[LAST_IDX - char_idx],
        UpToDown => STRETCH_V[char_idx],
        DownToUp => STRETCH_V[LAST_IDX - char_idx],
    };

    let is_reverse = matches!(motion, RightToLeft | DownToUp);

    let style = if is_reverse || char_idx == 0 {
        style
    } else {
        let s = Style::default()
            .add_modifier(style.add_modifier)
            .remove_modifier(style.sub_modifier);

        match (style.fg, style.bg) {
            (Some(fg), Some(bg)) => s.fg(bg).bg(fg),
            (Some(fg), None) => s.bg(fg),
            (None, Some(bg)) => s.fg(bg),
            (None, None) => s,
        }
    };

    (symbol, style)
}

const STRETCH_V: &[char; 8] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const STRETCH_H: &[char; 8] = &['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
const LAST_IDX: usize = STRETCH_H.len() - 1;

#[derive(Clone, Copy, Debug)]
struct StretchBounds {
    start: f32,
    end: f32,
}

impl StretchBounds {
    fn new(rect: Rect, motion: Motion, alpha: f32) -> StretchBounds {
        let (x, y) = (rect.x as f32, rect.y as f32);
        let (w, h) = (rect.width as f32, rect.height as f32);

        match motion {
            Motion::LeftToRight => StretchBounds { start: x, end: x + w * alpha },
            Motion::RightToLeft => StretchBounds {
                start: x + w - 1.0,
                end: x + (w - 1.0) * (1.0 - alpha),
            },
            Motion::UpToDown => StretchBounds { start: y, end: y + h * alpha },
            Motion::DownToUp => StretchBounds {
                start: y + h - 1.0,
                end: y + (h - 1.0) * (1.0 - alpha),
            },
        }
    }
}

impl Interpolatable for StretchBounds {
    fn lerp(&self, target: &Self, alpha: f32) -> Self {
        StretchBounds {
            start: self.start.lerp(&target.start, alpha),
            end: self.end.lerp(&target.end, alpha),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
    };

    use crate::{fx::stretch::Stretch, Duration, Effect, IntoEffect, Motion};

    fn assert_buf(buf: &Buffer, x: u16, y: u16, symbol: char, reverse: bool) {
        let style = if reverse || symbol == ' ' {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            Style::default().fg(Color::Black).bg(Color::White)
        };

        let cell = &buf[(x, y)];
        assert_eq!(cell.symbol(), symbol.to_string());
        assert_eq!(cell.style(), style);
    }

    fn stretch_fx(motion: Motion) -> Effect {
        Stretch::builder()
            .direction(motion)
            .timer(1000)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .build()
            .into_effect()
    }

    #[test]
    fn test_stretch_effect_left_to_right() {
        let mut fx = stretch_fx(Motion::LeftToRight);

        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        fx.process(
            Duration::from_secs_f32((2.0 / 3.0) + (0.7 / 3.0)),
            &mut buf,
            area,
        );

        assert_buf(&buf, 0, 0, ' ', false);
        assert_buf(&buf, 1, 0, ' ', false);
        assert_buf(&buf, 2, 0, '▊', false);
    }

    #[test]
    fn test_stretch_effect_right_to_left() {
        let mut fx = stretch_fx(Motion::RightToLeft);

        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        fx.process(
            Duration::from_secs_f32((2.0 / 3.0) + (0.7 / 3.0)),
            &mut buf,
            area,
        );

        // For RightToLeft, the stretch should start from the right edge
        // and move leftward, with the partial character having reversed colors
        assert_buf(&buf, 2, 0, ' ', true);
        assert_buf(&buf, 1, 0, ' ', true);
        assert_buf(&buf, 0, 0, '▎', true);
    }

    #[test]
    fn test_stretch_effect_up_to_down() {
        let mut fx = stretch_fx(Motion::UpToDown);

        let area = Rect::new(0, 0, 1, 3);
        let make_buffer = |area| Buffer::empty(area);

        let mut buf = make_buffer(area);

        fx.process(
            Duration::from_secs_f32((2.0 / 3.0) + (0.7 / 3.0)),
            &mut buf,
            area,
        );

        // For UpToDown, the stretch should fill from top and have partial character at y=2
        assert_buf(&buf, 0, 0, ' ', false);
        assert_buf(&buf, 0, 1, ' ', false);
        assert_buf(&buf, 0, 2, '▆', false);
    }

    #[test]
    fn test_stretch_effect_down_to_up() {
        let mut fx = stretch_fx(Motion::DownToUp);

        let area = Rect::new(0, 0, 1, 3);
        let make_buffer = |area| Buffer::empty(area);

        let mut buf = make_buffer(area);

        fx.process(
            Duration::from_secs_f32((2.0 / 3.0) + (0.7 / 3.0)),
            &mut buf,
            area,
        );

        // For DownToUp, the stretch should start from bottom and move upward
        assert_buf(&buf, 0, 0, '▂', true);
        assert_buf(&buf, 0, 1, ' ', true);
        assert_buf(&buf, 0, 2, ' ', true);
    }
}
