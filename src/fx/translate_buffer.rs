use std::cell::RefCell;
use std::rc::Rc;
use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Rect};

use crate::{BufferRenderer, CellFilter, CellIterator, Duration, EffectTimer, Interpolatable, Shader};

/// Translates the contents of an auxiliary buffer onto the main buffer.
///
/// This shader allows for efficient translation of pre-rendered content without
/// having to re-render it on every frame. It's particularly useful for large or
/// complex content that doesn't change frequently.
#[derive(Clone)]
pub struct TranslateBuffer {
    /// The auxiliary buffer containing the pre-rendered content to be translated.
    aux_buffer: Rc<RefCell<Buffer>>,
    /// The offset to translate the buffer by.
    translate_by: Offset,
    /// Timer controlling the duration and progress of the translation effect.
    timer: EffectTimer,
}

impl TranslateBuffer {
    /// Creates a new `TranslateBuffer` shader.
    ///
    /// # Arguments
    ///
    /// * `translate_by` - The final offset to translate the buffer by.
    /// * `timer` - The timer controlling the duration and interpolation of the effect.
    /// * `aux_buffer` - The auxiliary buffer containing the pre-rendered content.
    pub fn new(
        aux_buffer: Rc<RefCell<Buffer>>,
        translate_by: Offset,
        timer: EffectTimer,
    ) -> Self {
        Self {
            timer,
            aux_buffer,
            translate_by,
        }
    }
}

impl Shader for TranslateBuffer {
    fn name(&self) -> &'static str {
        "translate_by_buf"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        _area: Rect
    ) -> Option<Duration> {
        let overflow = self.timer.process(duration);
        let alpha = self.timer.alpha();

        let offset = Offset::default().lerp(&self.translate_by, alpha);
        self.aux_buffer.render_buffer(offset, buf);

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // Not used in this implementation
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        None
    }

    fn set_area(&mut self, _area: Rect) {

    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {
        // not applicable
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.timer.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::{Block, Borders, Widget};
    use crate::{CenteredShrink, Interpolation};

    fn translate_buffer_fx(translate_by: Offset) -> TranslateBuffer {
        let screen = Rect::new(0, 0, 20, 10);
        let aux_buffer = Rc::new(RefCell::new(Buffer::empty(screen)));
        TranslateBuffer::new(aux_buffer, translate_by, EffectTimer::from_ms(100, Interpolation::Linear))
    }

    fn assert_translation(
        translate_by: Offset,
        percent: u8,
        expected: Buffer,
    ) {
        assert_translation_fx(translate_buffer_fx(translate_by), percent, expected);
    }

    fn assert_translation_fx(
        mut fx: TranslateBuffer,
        percent: u8,
        expected: Buffer,
    ) {
        let screen = Rect::new(0, 0, 20, 10);
        let content = screen.inner_centered(10, 4);

        // Prepare the auxiliary buffer
        let mut aux_buffer = fx.aux_buffer.borrow_mut();
        let block = Block::default()
            .borders(Borders::ALL)
            .title("hello");
        block.render(content, &mut aux_buffer);
        drop(aux_buffer);

        let mut buf = Buffer::empty(screen);

        fx.process(Duration::from_millis(percent as _), &mut buf, content);

        assert_eq!(buf, expected)
    }

    #[test]
    fn test_translate_within_bounds() {
        assert_translation(Offset { x: 0, y: 3 }, 0, Buffer::with_lines([
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
        assert_translation(Offset { x: 0, y: 3 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: 0, y: -3 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: -5, y: -3 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: 5, y: 3 }, 100, Buffer::with_lines([
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
    fn test_translate_reversed() {
        let mut fx = translate_buffer_fx(Offset { x: -5, y: -3 });
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

        let mut fx = translate_buffer_fx(Offset { x: 5, y: 3 });
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
    fn test_translate_oob() {
        // down
        assert_translation(Offset { x: 0, y: 5 }, 100, Buffer::with_lines([
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "     ┌hello───┐     ",
            "     │        │     ",
        ]));
        assert_translation(Offset { x: 0, y: 6 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: 0, y: 7 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: 0, y: -5 }, 100, Buffer::with_lines([
            "     │        │     ",
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
        assert_translation(Offset { x: 0, y: -7 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: 7, y: 0 }, 100, Buffer::with_lines([
            "                    ",
            "                    ",
            "                    ",
            "            ┌hello──",
            "            │       ",
            "            │       ",
            "            └───────",
            "                    ",
            "                    ",
            "                    ",
        ]));
        assert_translation(Offset { x: 12, y: 0 }, 100, Buffer::with_lines([
            "                    ",
            "                    ",
            "                    ",
            "                 ┌he",
            "                 │  ",
            "                 │  ",
            "                 └──",
            "                    ",
            "                    ",
            "                    ",
        ]));
        assert_translation(Offset { x: 15, y: 0 }, 100, Buffer::with_lines([
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
        assert_translation(Offset { x: -7, y: 0 }, 100, Buffer::with_lines([
            "                    ",
            "                    ",
            "                    ",
            "ello───┐            ",
            "       │            ",
            "       │            ",
            "───────┘            ",
            "                    ",
            "                    ",
            "                    ",
        ]));
        assert_translation(Offset { x: -12, y: 0 }, 100, Buffer::with_lines([
            "                    ",
            "                    ",
            "                    ",
            "──┐                 ",
            "  │                 ",
            "  │                 ",
            "──┘                 ",
            "                    ",
            "                    ",
            "                    ",
        ]));
        assert_translation(Offset { x: -15, y: 0 }, 100, Buffer::with_lines([
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