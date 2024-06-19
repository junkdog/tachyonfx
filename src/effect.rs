use std::time::Duration;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Margin, Position, Rect};
use ratatui::style::Color;
use crate::{CellIterator, EffectTimer};
use crate::shader::Shader;

/// Represents an effect that can be applied to terminal cells.
/// The `Effect` struct wraps a shader, allowing it to be configured
/// and applied to a specified area and cell selection.
pub struct Effect {
    shader: Box<dyn Shader>,
}

impl Effect {
    /// Creates a new `Effect` with the specified shader.
    ///
    /// # Arguments
    /// * `shader` - The shader to be used for the effect. It must implement the `Shader` trait and have a static lifetime.
    ///
    /// # Returns
    /// * A new `Effect` instance.
    pub fn new<S>(shader: S) -> Self
        where S: Shader + 'static
    {
        Self { shader: Box::new(shader) }
    }

    /// Creates a new `Effect` with the specified area.
    ///
    /// # Arguments
    /// * `area` - The rectangular area where the effect will be applied.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified area.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::Effect;
    /// use ratatui::layout::Rect;
    ///
    /// let shader = MyShader::new();
    /// let effect = Effect::new(shader)
    ///     .with_area(Rect::new(0, 0, 10, 10));
    /// ```
    pub fn with_area(&self, area: Rect) -> Self {
        let mut cloned = self.clone();
        cloned.shader.set_area(area);
        cloned
    }

    /// Creates a new `Effect` with the specified cell selection mode.
    ///
    /// # Arguments
    /// * `mode` - The cell selection mode to be used for the effect.
    ///
    /// # Returns
    /// * A new `Effect` instance with the specified cell selection mode.
    ///
    /// # Example
    /// ```
    /// use tachyonfx::{Effect, CellFilter};
    ///
    /// let shader = MyShader::new();
    /// let effect = Effect::new(shader)
    ///     .with_cell_selection(CellFilter::Text);
    /// ```
    pub fn with_cell_selection(&self, mode: CellFilter) -> Self {
        let mut cloned = self.clone();
        cloned.set_cell_selection(mode);
        cloned
    }

    /// Creates a new `Effect` with the shader's reverse flag toggled.
    ///
    /// # Returns
    /// * A new `Effect` instance with the shader's reverse flag toggled.
    pub fn reversed(&self) -> Self {
        let mut cloned = self.clone();
        cloned.reverse();
        cloned
    }
}

/// A filter mode enables effects to operate on specific cells.
#[derive(Clone, Debug, Default)]
pub enum CellFilter {
    /// Selects every cell
    #[default]
    All,
    /// Selects cells with matching foreground color
    FgColor(Color),
    /// Selects cells with matching background color
    BgColor(Color),
    /// Selects cells within the inner margin of the area
    Inner(Margin),
    /// Selects cells outside the inner margin of the area
    Outer(Margin),
    /// Selects cells with text
    Text,
    /// Selects cells that match all the given filters
    AllOf(Vec<CellFilter>),
    /// Negates the given filter
    Not(Box<CellFilter>),
}

pub struct CellSelector {
    inner_area: Rect,
    strategy: CellFilter,
}

impl CellSelector {
    fn new(area: Rect, strategy: CellFilter) -> Self {
        let inner_area = Self::resolve_area(area, &strategy);

        Self { inner_area, strategy }
    }

    fn resolve_area(area: Rect, mode: &CellFilter) -> Rect {
        match mode {
            CellFilter::All           => area,
            CellFilter::Inner(margin) => area.inner(margin),
            CellFilter::Outer(margin) => area.inner(margin),
            CellFilter::Text          => area,
            CellFilter::AllOf(_)      => area,
            CellFilter::Not(m)        => Self::resolve_area(area, m.as_ref()),
            CellFilter::FgColor(_)    => area,
            CellFilter::BgColor(_)    => area,
        }
    }

    pub fn is_valid(&self, pos: Position, cell: &Cell) -> bool {
        let mode = &self.strategy;

        self.valid_position(pos, mode)
            && self.is_valid_cell(cell, mode)
    }

    fn valid_position(&self, pos: Position, mode: &CellFilter) -> bool {
        match mode {
            CellFilter::All        => self.inner_area.contains(pos),
            CellFilter::Inner(_)   => self.inner_area.contains(pos),
            CellFilter::Outer(_)   => !self.inner_area.contains(pos),
            CellFilter::Text       => self.inner_area.contains(pos),
            CellFilter::AllOf(s)   => s.iter()
                .all(|mode| mode.selector(self.inner_area).valid_position(pos, mode)),
            CellFilter::Not(m)  => self.valid_position(pos, m.as_ref()),
            CellFilter::FgColor(_) => self.inner_area.contains(pos),
            CellFilter::BgColor(_) => self.inner_area.contains(pos),
        }
    }

    fn is_valid_cell(&self, cell: &Cell, mode: &CellFilter) -> bool {
        match mode {
            CellFilter::Text => {
                if cell.symbol().len() == 1 {
                    let ch = cell.symbol().chars().next().unwrap();
                    ch.is_alphabetic() || ch.is_numeric() || ch == ' ' || "?!.,:;".contains(ch)
                } else {
                    false
                }
            },

            CellFilter::AllOf(s) => {
                s.iter()
                    .all(|s| s.selector(self.inner_area).is_valid_cell(cell, s))
            },

            CellFilter::FgColor(color) => cell.fg == *color,
            CellFilter::BgColor(color) => cell.bg == *color,

            CellFilter::Not(m) => !self.is_valid_cell(cell, m.as_ref()),

            _ => true,
        }
    }
}

impl CellFilter {
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

    fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator){
        self.shader.execute(alpha, area, cell_iter);
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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.shader.set_cell_selection(strategy)
    }

    fn reverse(&mut self) {
        self.shader.reverse()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        self.shader.timer_mut()
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.shader.cell_selection()
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