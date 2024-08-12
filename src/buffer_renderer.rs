use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Position};
use std::cell::RefCell;
use std::rc::Rc;

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
        blit_buffer(&*self.as_ref().borrow(), buf, offset);
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

#[cfg(test)]
mod tests {
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