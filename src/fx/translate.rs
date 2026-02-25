use alloc::boxed::Box;

use ratatui_core::{
    buffer::Buffer,
    layout::{Offset, Rect},
};

use crate::{
    bounding_box::BoundingBox, effect::Effect, effect_timer::EffectTimer,
    interpolation::Interpolatable, shader::Shader, CellFilter, ColorSpace, Duration,
};

#[derive(Clone, Debug)]
pub(super) struct Translate {
    fx: Effect,
    area: Option<Rect>,
    original_area: Option<BoundingBox>,
    translate_by: (f32, f32),
    timer: EffectTimer,
}

impl Translate {
    pub fn new(fx: Effect, translate_by: Offset, lifetime: EffectTimer) -> Self {
        let translate_by = (translate_by.x as f32, translate_by.y as f32);
        Self {
            fx,
            translate_by,
            timer: lifetime,
            area: None,
            original_area: None,
        }
    }
}

impl Shader for Translate {
    fn name(&self) -> &'static str {
        "translate_by"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let overflow = self.timer.process(duration);
        let alpha = self.timer.alpha();

        if self.original_area.is_none() {
            self.original_area = Some(BoundingBox::from_rect(area));
        }

        let (dx, dy) = (0.0, 0.0).lerp(&self.translate_by, alpha);
        let translated_area = self
            .original_area
            .as_ref()
            .map(|a| a.translate(dx, dy))
            .and_then(|a| a.as_rect(buf.area));

        self.area = translated_area;

        let fx_area = translated_area.unwrap_or_default();
        self.fx.set_area(fx_area);
        let hosted_overflow = self.fx.process(duration, buf, fx_area);

        // only return the overflow if the fx is done and this translate is done
        match (overflow, hosted_overflow) {
            (Some(a), Some(b)) => Some(a.min(b)),
            _ => None,
        }
    }

    fn done(&self) -> bool {
        self.timer.done() && self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
        self.fx.set_area(area);
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.fx.filter(strategy);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.fx.cell_filter()
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn reset(&mut self) {
        self.timer.reset();
        self.fx.reset();
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        let offset = Offset {
            x: self.translate_by.0 as i32,
            y: self.translate_by.1 as i32,
        };

        EffectExpression::parse(&format!(
            "fx::translate({}, {}, {})",
            self.fx.to_dsl()?,
            offset.dsl_format(),
            self.timer.dsl_format()
        ))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::{Block, Borders, Widget};

    use super::*;
    use crate::{CenteredShrink, Interpolation::Linear};

    fn assert_translation(translate_by: Offset, percent: u8, expected: &Buffer) {
        assert_translation_fx(translate_fx(translate_by), percent, expected);
    }

    fn translate_fx(translate_by: Offset) -> Translate {
        let fx = crate::fx::consume_tick();
        Translate::new(fx, translate_by, EffectTimer::from_ms(100, Linear))
    }

    fn assert_translation_fx(fx: Translate, percent: u8, expected: &Buffer) {
        let screen = Rect::new(0, 0, 20, 10);
        let content = screen.inner_centered(10, 4);

        let mut buf = Buffer::empty(screen);

        let mut fx = fx;
        fx.process(Duration::from_millis(percent as _), &mut buf, content);

        let block = Block::new().borders(Borders::ALL).title("hello");

        block.render(fx.area.unwrap(), &mut buf);

        assert_eq!(&buf, expected);
    }

    #[test]
    fn test_translate_within_bounds() {
        assert_translation(
            Offset { x: 0, y: 3 },
            0,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "     ┌hello───┐     ",
                "     │        │     ",
                "     │        │     ",
                "     └────────┘     ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: 0, y: 3 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "     ┌hello───┐     ",
                "     │        │     ",
                "     │        │     ",
                "     └────────┘     ",
            ]),
        );
        assert_translation(
            Offset { x: 0, y: -3 },
            100,
            &Buffer::with_lines([
                "     ┌hello───┐     ",
                "     │        │     ",
                "     │        │     ",
                "     └────────┘     ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: -5, y: -3 },
            100,
            &Buffer::with_lines([
                "┌hello───┐          ",
                "│        │          ",
                "│        │          ",
                "└────────┘          ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: 5, y: 3 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "          ┌hello───┐",
                "          │        │",
                "          │        │",
                "          └────────┘",
            ]),
        );
    }

    #[test]
    fn translate_reversed() {
        let mut fx = translate_fx(Offset { x: -5, y: -3 });
        fx.reverse();
        assert_translation_fx(
            fx,
            0,
            &Buffer::with_lines([
                "┌hello───┐          ",
                "│        │          ",
                "│        │          ",
                "└────────┘          ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );

        let mut fx = translate_fx(Offset { x: 5, y: 3 });
        fx.reverse();
        assert_translation_fx(
            fx,
            0,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "          ┌hello───┐",
                "          │        │",
                "          │        │",
                "          └────────┘",
            ]),
        );
    }

    #[test]
    fn translate_oob() {
        // down
        assert_translation(
            Offset { x: 0, y: 5 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "     ┌hello───┐     ",
                "     └────────┘     ",
            ]),
        );
        assert_translation(
            Offset { x: 0, y: 6 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "     ┌hello───┐     ",
            ]),
        );
        assert_translation(
            Offset { x: 0, y: 7 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );

        // up
        assert_translation(
            Offset { x: 0, y: -5 },
            100,
            &Buffer::with_lines([
                "     ┌hello───┐     ",
                "     └────────┘     ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: 0, y: -7 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );

        // right
        assert_translation(
            Offset { x: 7, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "            ┌hello─┐",
                "            │      │",
                "            │      │",
                "            └──────┘",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        // right
        assert_translation(
            Offset { x: 12, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                 ┌h┐",
                "                 │ │",
                "                 │ │",
                "                 └─┘",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: 15, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );

        // left
        assert_translation(
            Offset { x: -7, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "┌hello─┐            ",
                "│      │            ",
                "│      │            ",
                "└──────┘            ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: -12, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "┌h┐                 ",
                "│ │                 ",
                "│ │                 ",
                "└─┘                 ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
        assert_translation(
            Offset { x: -15, y: 0 },
            100,
            &Buffer::with_lines([
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
                "                    ",
            ]),
        );
    }
}
