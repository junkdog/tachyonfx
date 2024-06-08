use ratatui::layout::Rect;

pub trait CenteredShrink {
    fn inner_centered(&self, width: u16, height: u16) -> Rect;
}

impl CenteredShrink for Rect {
    fn inner_centered(&self, width: u16, height: u16) -> Rect {
        let x = self.x + (self.width.saturating_sub(width) / 2);
        let y = self.y + (self.height.saturating_sub(height) / 2);
        Rect::new(x, y, width.min(self.width), height.min(self.height))
    }
}
