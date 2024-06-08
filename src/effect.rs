use std::time::Duration;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Margin, Position, Rect};
use crate::shader::Shader;

pub struct Effect {
    shader: Box<dyn Shader>,
}

impl Effect {
    pub fn new<S>(shader: S) -> Self
        where S: Shader + 'static
    {
        Self { shader: Box::new(shader) }
    }

    pub fn with_area(&self, area: Rect) -> Self {
        let mut cloned = self.clone();
        cloned.shader.set_area(area);
        cloned
    }

    pub fn with_cell_selection(&self, mode: FilterMode) -> Self {
        let mut cloned = self.clone();
        cloned.cell_selection(mode);
        cloned
    }
}

#[derive(Clone, Debug, Default)]
pub enum FilterMode {
    #[default]
    All,
    Inner(Margin),
    Outer(Margin),
    Text,
    AllOf(Vec<FilterMode>),
    Negate(Box<FilterMode>),
}

pub struct CellSelector {
    inner_area: Rect,
    strategy: FilterMode,
}

impl CellSelector {
    fn new(area: Rect, strategy: FilterMode) -> Self {
        let inner_area = Self::resolve_area(area, &strategy);

        Self { inner_area, strategy }
    }

    fn resolve_area(area: Rect, mode: &FilterMode) -> Rect {
        match mode {
            FilterMode::All           => area,
            FilterMode::Inner(margin) => area.inner(margin),
            FilterMode::Outer(margin) => area.inner(margin),
            FilterMode::Text          => area,
            FilterMode::AllOf(_)      => area,
            FilterMode::Negate(m)     => Self::resolve_area(area, m.as_ref()),
        }
    }

    pub fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        let mode = &self.strategy;

        self.valid_position(pos, mode)
            && self.is_valid_cell(cell, mode)
    }

    pub fn is_valid_position(&self, pos: Position) -> bool {
        self.valid_position(pos, &self.strategy)
    }

    fn valid_position(&self, pos: Position, mode: &FilterMode) -> bool {
        match mode {
            FilterMode::All       => true,
            FilterMode::Inner(_)  => self.inner_area.contains(pos),
            FilterMode::Outer(_)  => !self.inner_area.contains(pos),
            FilterMode::Text      => true,
            FilterMode::AllOf(s)  => s.iter()
                .all(|mode| mode.selector(self.inner_area).valid_position(pos, mode)),
            FilterMode::Negate(m) => self.valid_position(pos, m.as_ref()),
        }
    }

    fn is_valid_cell(&self, cell: &Cell, mode: &FilterMode) -> bool {
        match mode {
            FilterMode::Text => {
                if cell.symbol().len() == 1 {
                    let ch = cell.symbol().chars().next().unwrap();
                    ch.is_alphabetic() || ch.is_numeric() || ch == ' ' || "?!.,:;".contains(ch)
                } else {
                    false
                }
            },

            FilterMode::AllOf(s) => {
                s.iter()
                    .all(|s| s.selector(self.inner_area).is_valid_cell(cell, s))
            },

            FilterMode::Negate(m) => !self.is_valid_cell(cell, m.as_ref()),

            _ => true,
        }
    }
}

impl FilterMode {
    pub fn selector(&self, area: Rect) -> CellSelector {
        CellSelector::new(area, self.clone())
    }
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self { shader: self.shader.clone_box() }
    }
}

impl Shader for Effect {
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let area = self.shader.area().unwrap_or(area);
        self.shader.process(duration, buf, area)
    }

    fn done(&self) -> bool {
        self.shader.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        self.shader.clone_box()
    }

    fn area(&self) -> Option<Rect> {
        self.shader.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.shader.set_area(area)
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        self.shader.cell_selection(strategy)
    }

    fn reverse(&mut self) {
        self.shader.reverse()
    }
}


pub trait IntoEffect {
    fn into_effect(self) -> Effect;
}

impl<S> IntoEffect for S
    where S: Shader + 'static
{
    fn into_effect(self) -> Effect {
        Effect::new(self)
    }
}