use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use crate::bounding_box::BoundingBox;
use crate::CellIterator;

use crate::effect::{Effect, CellFilter};
use crate::effect_timer::EffectTimer;
use crate::interpolation::Interpolatable;
use crate::shader::Shader;

#[derive(Clone, Default)]
pub struct Translate {
    fx: Option<Effect>,
    area: Option<Rect>,
    original: Option<BoundingBox>,
    translate_by: (f32, f32),
    timer: EffectTimer,
}

impl Translate {
    pub fn new(
        fx: Option<Effect>,
        translate_by: (i16, i16),
        lifetime: EffectTimer
    ) -> Self {
        let (dx, dy) = translate_by;
        let translate_by = (dx as f32, dy as f32);
        Self { fx, translate_by, timer: lifetime, ..Self::default() }
    }
}

impl Shader for Translate {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let overflow = self.timer.process(duration);
        let alpha = self.timer.alpha();

        if self.original.is_none() {
            self.original = Some(BoundingBox::from_rect(area));
        }

        let (dx, dy) = (0.0, 0.0).lerp(&self.translate_by, alpha);
        let translated_area = self.original.as_ref()
            .unwrap()
            .translate(dx, dy)
            .to_rect(buf.area);

        self.area = translated_area.clone();

        if let Some(fx) = &mut self.fx {
            let fx_area = translated_area.unwrap_or_default();
            fx.set_area(fx_area);
            fx.process(duration, buf, fx_area);
        }

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
    }

    fn done(&self) -> bool {
        self.timer.done()
            && self.fx.as_ref().map_or(true, Effect::done)
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
        if let Some(fx) = &mut self.fx {
            fx.set_area(area)
        }
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        if let Some(fx) = &mut self.fx {
            fx.set_cell_selection(strategy)
        }
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        if let Some(fx) = self.fx.as_ref() {
            return fx.cell_selection();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::{Block, Borders, Widget};
    use crate::{CenteredShrink, fx};
    use crate::Interpolation::Linear;
    use super::*;

    fn assert_translation(
        translate_by: (i16, i16),
        percent: u8,
        expected: Buffer,
    ) {
        assert_translation_fx(translate_fx(translate_by), percent, expected);
    }

    fn translate_fx(translate_by: (i16, i16)) -> Translate {
        Translate::new(None, translate_by, EffectTimer::from_ms(100, Linear))
    }

    fn assert_translation_fx(
        fx: Translate,
        percent: u8,
        expected: Buffer,
    ) {
        let screen = Rect::new(0, 0, 20, 10);
        let content = screen.inner_centered(10, 4);

        let mut buf = Buffer::empty(screen);

        let mut fx = fx.clone();
        fx.process(Duration::from_millis(percent as u64), &mut buf, content);

        let block = Block::new()
            .borders(Borders::ALL)
            .title("hello");

        block.render(fx.area.unwrap(), &mut buf);

        assert_eq!(buf, expected)
    }

    #[test]
    fn test_translate_within_bounds() {
        assert_translation((0, 3), 0, Buffer::with_lines([
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
        ]));
        assert_translation((0, 3), 100, Buffer::with_lines([
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
        ]));
        assert_translation((0, -3), 100, Buffer::with_lines([
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
        ]));
        assert_translation((-5, -3), 100, Buffer::with_lines([
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
        ]));
        assert_translation((5, 3), 100, Buffer::with_lines([
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
        ]));
    }

    #[test]
    fn translate_reversed() {
        let mut fx = translate_fx((-5, -3));
        fx.reverse();
        assert_translation_fx(fx, 0, Buffer::with_lines([
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
        ]));

        let mut fx = translate_fx((5, 3));
        fx.reverse();
        assert_translation_fx(fx, 0, Buffer::with_lines([
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
        ]));
    }

    #[test]
    fn translate_oob() {
        // down
        assert_translation((0, 5), 100, Buffer::with_lines([
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
        ]));
        assert_translation((0, 6), 100, Buffer::with_lines([
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
        ]));
        assert_translation((0, 7), 100, Buffer::with_lines([
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
        ]));

        // up
        assert_translation((0, -5), 100, Buffer::with_lines([
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
        ]));
        assert_translation((0, -7), 100, Buffer::with_lines([
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
        ]));

        // right
        assert_translation((7, 0), 100, Buffer::with_lines([
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
        ]));
        // right
        assert_translation((12, 0), 100, Buffer::with_lines([
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
        ]));
        assert_translation((15, 0), 100, Buffer::with_lines([
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
        ]));

        // left
        assert_translation((-7, 0), 100, Buffer::with_lines([
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
        ]));
        assert_translation((-12, 0), 100, Buffer::with_lines([
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
        ]));
        assert_translation((-15, 0), 100, Buffer::with_lines([
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
        ]));
    }
}