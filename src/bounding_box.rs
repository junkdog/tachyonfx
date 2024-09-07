use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    #[allow(dead_code)]
    pub(crate) fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub(crate) fn from_rect(rect: Rect) -> Self {
        Self {
            x: rect.x as f32,
            y: rect.y as f32,
            width: rect.width as f32,
            height: rect.height as f32,
        }
    }

    pub(crate) fn as_rect(&self, screen: Rect) -> Option<Rect> {
        match () {
            _ if self.x + self.width < screen.x as f32  => None,
            _ if self.y + self.height < screen.y as f32 => None,
            _ if self.x > (screen.x + screen.width) as f32 => None,
            _ if self.y > (screen.y + screen.height) as f32 => None,
            _ => {
                let dx: u16 = if self.x < 0.0 { self.x.round().abs() } else { 0.0 } as _;
                let dy: u16 = if self.y < 0.0 { self.y.round().abs() } else { 0.0 } as _;
                Some(Rect::new(
                    self.x.max(0.0).round() as u16,
                    self.y.max(0.0).round() as u16,
                    self.width.round() as u16 - dx,
                    self.height.round() as u16 - dy
                ))
            },
        }
    }

    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_rect_clamped() {
        let bb = BoundingBox::new(-1.0, -2.0, 3.0, 4.0);
        let screen = Rect::new(0, 0, 10, 10);
        assert_eq!(bb.as_rect(screen), Some(Rect::new(0, 0, 2, 2)));
    }

    #[test]
    fn test_to_rect_outside() {
        let bb = BoundingBox::new(-1.0, -2.0, 3.0, 4.0);
        let screen = Rect::new(5, 5, 10, 10);
        assert_eq!(bb.as_rect(screen), None);
    }

    #[test]
    fn test_translate() {
        let bb = BoundingBox::new(1.0, 2.0, 3.0, 4.0);
        let bb = bb.translate(5.0, 6.0);
        assert_eq!(bb.x, 6.0);
        assert_eq!(bb.y, 8.0);
        assert_eq!(bb.width, 3.0);
        assert_eq!(bb.height, 4.0);
    }
}