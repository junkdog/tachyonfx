use std::cell::RefCell;
use std::rc::Rc;
use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Position};
use ratatui::style::{Color, Modifier, Style};

/// A trait for rendering the contents of one buffer onto another.
///
/// This trait is primarily implemented for `Rc<RefCell<Buffer>>`, allowing
/// for efficient rendering of one buffer's contents onto another at a specified offset.
/// This is useful for composing complex UI layouts or implementing effects that involve
/// rendering one buffer onto another.
///
/// # Safety
///
/// The implementation ensures that it does not write outside the bounds
/// of the provided buffer. The `offset` parameter is used to correctly
/// position the rendered content within the target buffer.
pub trait BufferRenderer {

    /// Renders the contents of this buffer onto the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `offset` - The position offset at which to start rendering in the target buffer.
    /// * `buf` - The target buffer to render onto.
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer);
}

impl BufferRenderer for Rc<RefCell<Buffer>> {
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer) {
        (&*self.as_ref().borrow())
            .render_buffer(offset, buf);
    }
}

impl BufferRenderer for Buffer {
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer) {
        blit_buffer(self, buf, offset);
    }
}

/// Copies the contents of a source buffer onto a destination buffer with a specified offset.
///
/// This function performs a "blit" operation, copying cells from the source buffer to the
/// destination buffer. It handles clipping on all edges, ensuring that only the overlapping
/// region is copied. The function also correctly handles negative offsets.
///
/// # Arguments
///
/// * `src` - The source buffer to copy from.
/// * `dst` - The destination buffer to copy into. This buffer is modified in-place.
/// * `offset` - The offset at which to place the top-left corner of the source buffer
///              relative to the destination buffer. Can be negative.
///
/// # Behavior
///
/// - If the offset would place the entire source buffer outside the bounds of the
///   destination buffer, no copying occurs.
/// - The function clips the source buffer as necessary to fit within the destination buffer.
/// - Negative offsets are handled by adjusting the starting position in the source buffer.
pub fn blit_buffer(
    src: &Buffer,
    dst: &mut Buffer,
    offset: Offset,
) {
    let mut aux_area = src.area; // guaranteed to be Some
    aux_area.x = offset.x.max(0) as _;
    aux_area.y = offset.y.max(0) as _;

    let target_area = dst.area().clone();

    let l_clip_x: u16 = offset.x.min(0).abs() as _;
    let l_clip_y: u16 = offset.y.min(0).abs() as _;

    let r_clip_x: u16 = aux_area.x + aux_area.width - l_clip_x;
    let r_clip_x: u16 = r_clip_x - r_clip_x.min(target_area.width);

    let r_clip_y: u16 = aux_area.y + aux_area.height - l_clip_y;
    let r_clip_y: u16 = r_clip_y - r_clip_y.min(target_area.height);

    if aux_area.width.checked_sub(r_clip_x).is_none()
        || aux_area.height.checked_sub(r_clip_y).is_none()
    {
        return;
    }

    for y in l_clip_y..(aux_area.height - r_clip_y) {
        for x in l_clip_x..(aux_area.width - r_clip_x) {
            if let (Some(c), Some(new_c)) = (
                dst.cell_mut(Position::new(
                    x + aux_area.x - l_clip_x,
                    y + aux_area.y - l_clip_y,
                )),
                src.cell(Position::new(x, y)),
            ) {
                *c = new_c.clone();
            }
        }
    }
}

/// Converts a `Buffer` to an ANSI-encoded string representation.
///
/// This function takes a `Buffer` and converts it to a string that includes ANSI escape codes
/// for styling. The resulting string represents the content of the buffer with all styling
/// information (colors and text modifiers) preserved.
///
/// # Arguments
///
/// * `buffer` - A reference to the `Buffer` to be converted.
///
/// # Returns
///
/// A `String` containing the styled representation of the buffer's content.
pub fn render_as_ansi_string(buffer: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell(Position::new(x, y)).unwrap();
            s.push_str(&escape_code_of(cell.style()));
            s.push_str(cell.symbol());
            s.push_str("\x1b[0m"); // reset
        }
        s.push_str("\n");
    }
    s
}

fn escape_code_of(style: Style) -> String {
    let mut result = String::new();

    // Foreground color
    if let Some(color) = style.fg {
        if color != Color::Reset {
            result.push_str(&color_code(color, true));
        }
    }

    // Background color
    if let Some(color) = style.bg {
        if color != Color::Reset {
            result.push_str(&color_code(color, false));
        }
    }

    // Modifiers
    if style.add_modifier.contains(Modifier::BOLD) {
        result.push_str("\x1b[1m");
    }
    if style.add_modifier.contains(Modifier::DIM) {
        result.push_str("\x1b[2m");
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        result.push_str("\x1b[3m");
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        result.push_str("\x1b[4m");
    }
    if style.add_modifier.contains(Modifier::SLOW_BLINK) {
        result.push_str("\x1b[5m");
    }
    if style.add_modifier.contains(Modifier::RAPID_BLINK) {
        result.push_str("\x1b[6m");
    }
    if style.add_modifier.contains(Modifier::REVERSED) {
        result.push_str("\x1b[7m");
    }
    if style.add_modifier.contains(Modifier::HIDDEN) {
        result.push_str("\x1b[8m");
    }
    if style.add_modifier.contains(Modifier::CROSSED_OUT) {
        result.push_str("\x1b[9m");
    }

    result
}

fn color_code(color: Color, foreground: bool) -> String {
    let base = if foreground { 38 } else { 48 };
    match color {
        Color::Reset        => "\x1b[0m".to_string(),
        Color::Black        => format!("\x1b[{};5;0m", base),
        Color::Red          => format!("\x1b[{};5;1m", base),
        Color::Green        => format!("\x1b[{};5;2m", base),
        Color::Yellow       => format!("\x1b[{};5;3m", base),
        Color::Blue         => format!("\x1b[{};5;4m", base),
        Color::Magenta      => format!("\x1b[{};5;5m", base),
        Color::Cyan         => format!("\x1b[{};5;6m", base),
        Color::Gray         => format!("\x1b[{};5;7m", base),
        Color::DarkGray     => format!("\x1b[{};5;8m", base),
        Color::LightRed     => format!("\x1b[{};5;9m", base),
        Color::LightGreen   => format!("\x1b[{};5;10m", base),
        Color::LightYellow  => format!("\x1b[{};5;11m", base),
        Color::LightBlue    => format!("\x1b[{};5;12m", base),
        Color::LightMagenta => format!("\x1b[{};5;13m", base),
        Color::LightCyan    => format!("\x1b[{};5;14m", base),
        Color::White        => format!("\x1b[{};5;15m", base),
        Color::Indexed(i)   => format!("\x1b[{};5;{}m", base, i),
        Color::Rgb(r, g, b) => format!("\x1b[{};2;{};{};{}m", base, r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use super::*;

    fn assert_buffer_to_buffer_copy(
        offset: Offset,
        expected: Buffer,
    ) {

        let aux_buffer = Rc::new(RefCell::new(Buffer::with_lines([
            "abcd",
            "efgh",
            "ijkl",
            "mnop",
        ])));

        let mut buf = Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
            ". . . . ",
        ]);

        aux_buffer.render_buffer(offset, &mut buf);

        assert_eq!(buf, expected)
    }

    #[test]
    fn test_render_offsets_in_bounds() {
        assert_buffer_to_buffer_copy(
            Offset { x: 0, y: 0 },
            Buffer::with_lines([
                "abcd. . ",
                "efgh. . ",
                "ijkl. . ",
                "mnop. . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
            ])
        );

        assert_buffer_to_buffer_copy(
            Offset { x: 4, y: 3 },
            Buffer::with_lines([
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . abcd",
                ". . efgh",
                ". . ijkl",
                ". . mnop",
                ". . . . ",
            ])
        );
    }

    #[test]
    fn test_render_offsets_out_of_bounds() {
        assert_buffer_to_buffer_copy(
            Offset { x: -1, y: -2 },
            Buffer::with_lines([
                "jkl . . ",
                "nop . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
            ])
        );
        assert_buffer_to_buffer_copy(
            Offset { x: 6, y: 6 },
            Buffer::with_lines([
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . . ",
                ". . . ab",
                ". . . ef",
            ])
        );
    }

    #[test]
    fn test_render_from_larger_aux_buffer() {
        let aux_buffer = Rc::new(RefCell::new(Buffer::with_lines([
            "AAAAAAAAAA",
            "BBBBBBBBBB",
            "CCCCCCCCCC",
            "DDDDDDDDDD",
            "EEEEEEEEEE",
            "FFFFFFFFFF",
        ])));

        let buffer = || Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            ". . . . ",
        ]);

        // Test with no vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset::default(), &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            "AAAAAAAA",
            "BBBBBBBB",
            "CCCCCCCC",
        ]));

        // Test with positive vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: 2 }, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            "AAAAAAAA",
        ]));

        // Test with negative vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: -2 }, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            "CCCCCCCC",
            "DDDDDDDD",
            "EEEEEEEE",
        ]));

        // Test with both horizontal and vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 2, y: 1 }, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            ". . . . ",
            ". AAAAAA",
            ". BBBBBB",
        ]));

        // Test with out-of-bounds vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: 6 }, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            ". . . . ",
            ". . . . ",
            ". . . . ",
        ]));

        // Test with large negative vertical and horizontal offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: -5, y: -5 }, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            "FFFFF . ",
            ". . . . ",
            ". . . . ",
        ]));
    }
}