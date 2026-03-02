use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};
use core::cell::RefCell;

use ratatui_core::{
    buffer::{Buffer, Cell},
    layout::{Offset, Position, Rect},
    style::{Color, Modifier, Style},
};

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

    fn render_buffer_region(&self, src_region: Rect, offset: Offset, buf: &mut Buffer);
}

impl BufferRenderer for Rc<RefCell<Buffer>> {
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer) {
        (*self.as_ref().borrow()).render_buffer(offset, buf);
    }

    fn render_buffer_region(&self, src_region: Rect, offset: Offset, buf: &mut Buffer) {
        (*self.as_ref().borrow()).render_buffer_region(src_region, offset, buf);
    }
}

#[cfg(feature = "sendable")]
impl BufferRenderer for crate::RefCount<Buffer> {
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer) {
        (*self.lock().unwrap()).render_buffer(offset, buf);
    }

    fn render_buffer_region(&self, src_region: Rect, offset: Offset, buf: &mut Buffer) {
        (*self.lock().unwrap()).render_buffer_region(src_region, offset, buf);
    }
}

impl BufferRenderer for Buffer {
    fn render_buffer(&self, offset: Offset, buf: &mut Buffer) {
        blit_buffer(self, buf, offset);
    }

    fn render_buffer_region(&self, src_region: Rect, offset: Offset, buf: &mut Buffer) {
        blit_buffer_region(self, src_region, buf, offset);
    }
}

/// Copies the contents of a source buffer onto a destination buffer with a specified
/// offset.
///
/// This function performs a "blit" operation, copying cells from the source buffer to the
/// destination buffer. It handles clipping on all edges, ensuring that only the
/// overlapping region is copied. The function also correctly handles negative offsets.
///
/// # Arguments
///
/// * `src` - The source buffer to copy from.
/// * `dst` - The destination buffer to copy into. This buffer is modified in-place.
/// * `offset` - The offset at which to place the top-left corner of the source buffer
///   relative to the destination buffer. Can be negative.
///
/// # Behavior
///
/// - Individual cells marked with `skip = true` in the source buffer are not copied,
///   leaving the destination cells unchanged.
/// - If the offset would place the entire source buffer outside the bounds of the
///   destination buffer, no copying occurs.
/// - The function clips the source buffer as necessary to fit within the destination
///   buffer.
/// - Negative offsets are handled by adjusting the starting position in the source
///   buffer.
pub fn blit_buffer(src: &Buffer, dst: &mut Buffer, offset: Offset) {
    blit_buffer_region(src, src.area, dst, offset);
}

/// Copies the specified region of a source buffer onto a destination buffer with a
/// specified offset.
///
/// This function performs a "blit" operation, copying cells from the source buffer to the
/// destination buffer. It handles clipping on all edges, ensuring that only the
/// overlapping region is copied. The function also correctly handles negative offsets.
///
/// # Arguments
///
/// * `src` - The source buffer to copy from.
/// * `src_region` - The rectangular region within the source buffer to copy. This region
///   will be automatically clipped to the source buffer's bounds.
/// * `dst` - The destination buffer to copy into. This buffer is modified in-place.
/// * `offset` - The offset at which to place the top-left corner of the source region
///   relative to the destination buffer. Can be negative.
///
/// # Behavior
///
/// - The source region is automatically clipped to the bounds of the source buffer.
/// - Individual cells marked with `skip = true` in the source buffer are not copied,
///   leaving the destination cells unchanged.
/// - If the offset would place the entire source buffer outside the bounds of the
///   destination buffer, no copying occurs.
/// - The function clips the source region as necessary to fit within the destination
///   buffer.
/// - Negative offsets are handled by adjusting the starting position in the source
///   buffer.
pub fn blit_buffer_region(src: &Buffer, src_region: Rect, dst: &mut Buffer, offset: Offset) {
    #[inline(always)]
    fn should_copy_cell(cell: &Cell) -> bool {
        #[cfg(not(feature = "ratatui-next-cell"))]
        return !cell.skip;

        #[cfg(feature = "ratatui-next-cell")]
        return cell.diff_option != ratatui_core::buffer::CellDiffOption::Skip;
    }

    // clip source region to source buffer bounds
    let src_region = src_region.intersection(src.area);

    let clip = ClipRegion::new(src_region, *dst.area(), offset);
    if !clip.is_valid() {
        return; // zero area or out of bounds
    }

    // copy cells from clipped source region to destination buffer
    for y in 0..clip.height() {
        for x in 0..clip.width() {
            let base_pos = Position::new(x, y);
            let src_cell = &src[clip.src_pos(base_pos)];

            if should_copy_cell(src_cell) {
                dst[clip.dst_pos(base_pos)] = src_cell.clone();
            }
        }
    }
}

/// Converts a `Buffer` to an ANSI-encoded string representation.
///
/// This function takes a `Buffer` and converts it to a string that includes ANSI escape
/// codes for styling. The resulting string represents the content of the buffer with all
/// styling information (colors and text modifiers) preserved.
///
/// This implementation properly handles unicode characters that span multiple cells by:
/// - Detecting and skipping space cells that follow multi-width characters
/// - Preserving the correct visual representation of unicode text without extra spaces
///
/// # Arguments
///
/// * `buffer` - A reference to the `Buffer` to be converted.
///
/// # Returns
///
/// A `String` containing the styled representation of the buffer's content.
#[deprecated(
    since = "0.16.0",
    note = "use `buffer_to_ansi_string(buffer, false)` instead"
)]
pub fn render_as_ansi_string(buffer: &Buffer) -> String {
    buffer_to_ansi_string(buffer, false)
}

/// Converts a `Buffer` to an ANSI-encoded string representation with configurable width
/// handling.
///
/// This function takes a `Buffer` and converts it to a string that includes ANSI escape
/// codes for styling. The resulting string represents the content of the buffer with all
/// styling information (colors and text modifiers) preserved.
///
/// # Arguments
///
/// * `buffer` - A reference to the `Buffer` to be converted.
/// * `include_all_cells` - If `true`, includes every cell in the buffer grid, even those
///   that are spaces following multi-width characters. If `false`, properly handles
///   unicode characters by detecting and skipping space cells that follow multi-width
///   characters to avoid extra spaces in the output.
///
/// # Returns
///
/// A `String` containing the styled representation of the buffer's content.
///
/// # Cell Handling Modes
///
/// When `include_all_cells` is `false` (default):
/// - Skips space cells that follow multi-width characters (emoji, CJK characters, etc.)
/// - Produces compact output suitable for display terminals
/// - Example: "🦀test" renders as "🦀test" (no extra spaces)
///
/// When `include_all_cells` is `true`:
/// - Includes every cell in the buffer grid, regardless of character width
/// - Useful for output formats that expect the full buffer grid to be printed
/// - Example: "🦀test" might render as "🦀 test" (with spaces from the buffer grid)
pub fn buffer_to_ansi_string(buffer: &Buffer, include_all_cells: bool) -> String {
    use unicode_width::UnicodeWidthStr;

    let mut s = String::new();
    let mut style = Style::default();

    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let cell = buffer.cell(Position::new(x, y)).unwrap();

            // Skip cells that are spaces following a multi-width character
            // to avoid extra spaces in unicode output (unless include_all_cells is true)
            if !include_all_cells && cell.symbol() == " " && x > 0 {
                if let Some(prev_cell) = buffer.cell(Position::new(x - 1, y)) {
                    if prev_cell.symbol().width() > 1 {
                        x += 1;
                        continue;
                    }
                }
            }

            if cell.style() != style {
                s.push_str("\x1b[0m"); // reset
                s.push_str(&escape_code_of(cell.style()));
                style = cell.style();
            }

            s.push_str(cell.symbol());
            x += 1;
        }
        s.push_str("\x1b[0m");
        s.push('\n');

        // need to reset the style at the end of each line,
        // so that the style correctly carries over to the next line
        style = Style::default();
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
        Color::Reset => "\x1b[0m".to_string(),
        Color::Black => format!("\x1b[{base};5;0m"),
        Color::Red => format!("\x1b[{base};5;1m"),
        Color::Green => format!("\x1b[{base};5;2m"),
        Color::Yellow => format!("\x1b[{base};5;3m"),
        Color::Blue => format!("\x1b[{base};5;4m"),
        Color::Magenta => format!("\x1b[{base};5;5m"),
        Color::Cyan => format!("\x1b[{base};5;6m"),
        Color::Gray => format!("\x1b[{base};5;7m"),
        Color::DarkGray => format!("\x1b[{base};5;8m"),
        Color::LightRed => format!("\x1b[{base};5;9m"),
        Color::LightGreen => format!("\x1b[{base};5;10m"),
        Color::LightYellow => format!("\x1b[{base};5;11m"),
        Color::LightBlue => format!("\x1b[{base};5;12m"),
        Color::LightMagenta => format!("\x1b[{base};5;13m"),
        Color::LightCyan => format!("\x1b[{base};5;14m"),
        Color::White => format!("\x1b[{base};5;15m"),
        Color::Indexed(i) => format!("\x1b[{base};5;{i}m"),
        Color::Rgb(r, g, b) => format!("\x1b[{base};2;{r};{g};{b}m"),
    }
}

/// Helper struct to handle clipping calculations
struct ClipRegion {
    src: Rect,
    dst: Rect,
}

impl ClipRegion {
    fn new(src_region: Rect, dst_bounds: Rect, dst_offset: Offset) -> Self {
        let x_offset = dst_offset.x.min(0).unsigned_abs() as u16;
        let y_offset = dst_offset.y.min(0).unsigned_abs() as u16;

        let dst = Rect::new(
            dst_offset.x.max(0) as u16,
            dst_offset.y.max(0) as u16,
            src_region.width,
            src_region.height,
        );

        // adjust source and destination regions based on clipping and bounds
        let width = (dst.width - x_offset)
            .min(dst_bounds.width.saturating_sub(dst.x))
            .min(src_region.width);

        let height = (dst.height - y_offset)
            .min(dst_bounds.height.saturating_sub(dst.y))
            .min(src_region.height);

        Self {
            src: Rect::new(
                src_region.x + x_offset,
                src_region.y + y_offset,
                width,
                height,
            ),
            dst: Rect::new(dst.x, dst.y, width, height),
        }
    }

    fn is_valid(&self) -> bool {
        self.src.area() > 0
    }

    fn width(&self) -> u16 {
        self.src.width
    }

    fn height(&self) -> u16 {
        self.src.height
    }

    fn src_pos(&self, pos: Position) -> Position {
        Position::new(self.src.x + pos.x, self.src.y + pos.y)
    }

    fn dst_pos(&self, pos: Position) -> Position {
        Position::new(self.dst.x + pos.x, self.dst.y + pos.y)
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::buffer::Buffer;

    use super::*;
    use crate::ref_count;

    fn assert_buffer_to_buffer_copy(offset: Offset, expected: &Buffer) {
        let aux_buffer = ref_count(Buffer::with_lines(["abcd", "efgh", "ijkl", "mnop"]));

        let mut buf = Buffer::with_lines([
            ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ",
            ". . . . ",
        ]);

        aux_buffer.render_buffer(offset, &mut buf);

        assert_eq!(&buf, expected);
    }

    #[test]
    fn test_render_offsets_in_bounds() {
        assert_buffer_to_buffer_copy(
            Offset { x: 0, y: 0 },
            &Buffer::with_lines([
                "abcd. . ", "efgh. . ", "ijkl. . ", "mnop. . ", ". . . . ", ". . . . ", ". . . . ",
                ". . . . ",
            ]),
        );

        assert_buffer_to_buffer_copy(
            Offset { x: 4, y: 3 },
            &Buffer::with_lines([
                ". . . . ", ". . . . ", ". . . . ", ". . abcd", ". . efgh", ". . ijkl", ". . mnop",
                ". . . . ",
            ]),
        );
    }

    #[test]
    fn test_render_offsets_out_of_bounds() {
        assert_buffer_to_buffer_copy(
            Offset { x: -1, y: -2 },
            &Buffer::with_lines([
                "jkl . . ", "nop . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ",
                ". . . . ",
            ]),
        );
        assert_buffer_to_buffer_copy(
            Offset { x: 6, y: 6 },
            &Buffer::with_lines([
                ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . ab",
                ". . . ef",
            ]),
        );
    }

    #[test]
    fn test_render_from_larger_aux_buffer() {
        let aux_buffer = ref_count(Buffer::with_lines([
            "AAAAAAAAAA",
            "BBBBBBBBBB",
            "CCCCCCCCCC",
            "DDDDDDDDDD",
            "EEEEEEEEEE",
            "FFFFFFFFFF",
        ]));

        let buffer = || Buffer::with_lines([". . . . ", ". . . . ", ". . . . "]);

        // Test with no vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset::default(), &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines(["AAAAAAAA", "BBBBBBBB", "CCCCCCCC",])
        );

        // Test with positive vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: 2 }, &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", "AAAAAAAA",])
        );

        // Test with negative vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: -2 }, &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines(["CCCCCCCC", "DDDDDDDD", "EEEEEEEE",])
        );

        // Test with both horizontal and vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 2, y: 1 }, &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". AAAAAA", ". BBBBBB",])
        );

        // Test with out-of-bounds vertical offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: 0, y: 6 }, &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", ". . . . ",])
        );

        // Test with large negative vertical and horizontal offset
        let mut buf = buffer();
        aux_buffer.render_buffer(Offset { x: -5, y: -5 }, &mut buf);
        assert_eq!(
            buf,
            Buffer::with_lines(["FFFFF . ", ". . . . ", ". . . . ",])
        );
    }

    #[test]
    fn test_buffer_to_ansi_string_unicode() {
        // Test that set_stringn works properly with unicode and ANSI output doesn't have extra
        // spaces
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_stringn(0, 0, "🦀test", 8, Style::default());

        let ansi_output = buffer_to_ansi_string(&buffer, false);

        // Should contain both the emoji and the text directly adjacent (no extra spaces)
        assert!(ansi_output.contains("🦀test"));

        // Test with CJK characters
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_stringn(0, 0, "世界test", 8, Style::default());

        let ansi_output = buffer_to_ansi_string(&buffer, false);
        // Should be directly adjacent, no extra spaces between wide characters
        assert!(ansi_output.contains("世界test"));

        // Test with styled unicode
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
        buffer.set_stringn(0, 0, "🦀", 2, Style::default().fg(Color::Red));
        buffer.set_stringn(2, 0, "test", 4, Style::default().fg(Color::Blue));

        let ansi_output = buffer_to_ansi_string(&buffer, false);
        assert!(ansi_output.contains("🦀"));
        assert!(ansi_output.contains("test"));
        // Should contain both red and blue color codes
        assert!(ansi_output.contains("\x1b[38;5;1m")); // Red
        assert!(ansi_output.contains("\x1b[38;5;4m")); // Blue
        assert!(ansi_output.contains("\x1b[0m")); // Reset codes

        // Test edge case: multi-width at end of line
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
        buffer.set_stringn(0, 0, "a🦀", 3, Style::default());

        let ansi_output = buffer_to_ansi_string(&buffer, false);
        assert!(ansi_output.contains("a🦀"));
        assert!(!ansi_output.contains("a🦀 ")); // No trailing space
    }

    #[test]
    fn test_buffer_to_ansi_string_spacing_demo() {
        // Demonstrate the issue and solution with a clear example
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
        buffer.set_stringn(0, 0, "🦀🐍🌟hello", 12, Style::default());

        let ansi_output = buffer_to_ansi_string(&buffer, false);

        // Without proper width handling, this would be "🦀 🐍 🌟 hello" with spaces
        // With proper width handling, this should be "🦀🐍🌟hello" without extra spaces
        assert!(ansi_output.contains("🦀🐍🌟hello"));
        assert!(!ansi_output.contains("🦀 🐍")); // No spaces between emojis
        assert!(!ansi_output.contains("🐍 🌟")); // No spaces between emojis
        assert!(!ansi_output.contains("🌟 hello")); // No space before hello
    }

    #[test]
    fn test_buffer_to_ansi_string_include_all_cells() {
        // Test the include_all_cells option
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_stringn(0, 0, "🦀test", 8, Style::default());

        // Default behavior: skip spaces after wide characters
        let ansi_output_default = buffer_to_ansi_string(&buffer, false);
        assert!(ansi_output_default.contains("🦀test"));

        // Include all cells: include all cells from the buffer grid
        let ansi_output_all_cells = buffer_to_ansi_string(&buffer, true);

        // With include_all_cells=true, we should get the space that follows the emoji
        // The exact output depends on how ratatui's set_stringn handles the wide character
        assert!(ansi_output_all_cells.contains("🦀"));
        assert!(ansi_output_all_cells.contains("test"));

        // Test with CJK characters
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_stringn(0, 0, "世界", 4, Style::default());

        let ansi_output_default = buffer_to_ansi_string(&buffer, false);
        let ansi_output_all_cells = buffer_to_ansi_string(&buffer, true);

        // Both should contain the characters, but all_cells might have spaces
        assert!(ansi_output_default.contains("世界"));
        // Note: The all_cells version might have the characters split by spaces
        // so we test for individual characters
        assert!(ansi_output_all_cells.contains("世") || ansi_output_all_cells.contains("界"));

        // The all_cells version should be longer or equal (includes more spaces)
        assert!(ansi_output_all_cells.len() >= ansi_output_default.len());
    }

    #[test]
    fn test_blit_buffer_region() {
        let buffer =
            || Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . "]);

        let aux_buffer = Buffer::with_lines(["abcd", "efgh", "ijkl", "mnop"]);

        let mut buf = buffer();
        blit_buffer_region(
            &aux_buffer,
            Rect::new(1, 1, 2, 2),
            &mut buf,
            Offset::default(),
        );
        assert_eq!(
            buf,
            Buffer::with_lines(["fg. . . ", "jk. . . ", ". . . . ", ". . . . ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(&aux_buffer, Rect::new(1, 1, 2, 2), &mut buf, Offset {
            x: 4,
            y: 2,
        });
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", ". . fg. ", ". . jk. ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(&aux_buffer, Rect::new(1, 1, 3, 3), &mut buf, Offset {
            x: -1,
            y: -1,
        });
        assert_eq!(
            buf,
            Buffer::with_lines(["kl. . . ", "op. . . ", ". . . . ", ". . . . ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(
            &aux_buffer,
            Rect::new(2, 2, 3, 3),
            &mut buf,
            Offset::default(),
        );
        assert_eq!(
            buf,
            Buffer::with_lines(["kl. . . ", "op. . . ", ". . . . ", ". . . . ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(&aux_buffer, Rect::new(0, 0, 2, 2), &mut buf, Offset {
            x: 6,
            y: 3,
        });
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . ab", ". . . ef",])
        );

        let mut buf = buffer();
        blit_buffer_region(&aux_buffer, Rect::new(0, 0, 2, 2), &mut buf, Offset {
            x: 8,
            y: 8,
        });
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(
            &aux_buffer,
            Rect::new(1, 1, 0, 0),
            &mut buf,
            Offset::default(),
        );
        assert_eq!(
            buf,
            Buffer::with_lines([". . . . ", ". . . . ", ". . . . ", ". . . . ", ". . . . ",])
        );

        let mut buf = buffer();
        blit_buffer_region(
            &aux_buffer,
            Rect::new(0, 0, 4, 4),
            &mut buf,
            Offset::default(),
        );
        assert_eq!(
            buf,
            Buffer::with_lines(["abcd. . ", "efgh. . ", "ijkl. . ", "mnop. . ", ". . . . ",])
        );
    }
}
