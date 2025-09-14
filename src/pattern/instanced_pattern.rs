use ratatui::layout::Position;

pub(crate) trait InstancedPattern {
    fn map_alpha(&mut self, pos: Position) -> f32;
}
