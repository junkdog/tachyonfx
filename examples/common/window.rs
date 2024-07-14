use std::time::Duration;

use derive_builder::Builder;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, BorderType};
use ratatui::widgets::Widget;

use tachyonfx::{CellFilter, CellIterator, Effect, EffectTimer, IntoEffect, Shader};

#[derive(Builder, Clone)]
#[builder(pattern = "owned")]
pub struct OpenWindow {
    title: Line<'static>,
    #[builder(default, setter(strip_option))]
    pre_render_fx: Option<Effect>, // for setting up geometry etc
    #[builder(default, setter(strip_option))]
    parent_window_fx: Option<Effect>, // applied to whole buffer
    #[builder(default, setter(strip_option))]
    content_fx: Option<Effect>, // applied to content area
    title_style: Style,
    border_style: Style,
    border_type: BorderType,
    background: Style,
    borders: Borders,
}

impl From<OpenWindowBuilder> for Effect {
    fn from(value: OpenWindowBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

impl OpenWindow {
    pub fn builder() -> OpenWindowBuilder {
        OpenWindowBuilder::default()
    }

    pub fn screen_area(&mut self, area: Rect) {
        if let Some(fx) = self.parent_window_fx.as_mut() {
            fx.set_area(area);
        }
    }

    fn window_block(&self) -> Block {
        Block::new()
            .borders(Borders::ALL)
            .title_style(self.title_style)
            .title(self.title.clone())
            .border_style(self.border_style)
            .borders(self.borders)
            .border_type(self.border_type)
            .style(self.background)
    }

    pub fn processing_content_fx(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) {
        if let Some(fx) = self.content_fx.as_mut() {
            if fx.running() {
                fx.process(duration, buf, area);
            }
        }
    }
}

impl Shader for OpenWindow {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        if let Some(parent_window_fx) = self.parent_window_fx.as_mut() {
            parent_window_fx.process(duration, buf, area);
            if parent_window_fx.done() {
                self.parent_window_fx = None;
            }
        }

        let overflow = match self.pre_render_fx.as_mut() {
            Some(fx) if fx.running() => fx.process(duration, buf, area),
            _                        => Some(duration)
        };

        let area = if let Some(fx) = self.pre_render_fx.as_ref() {
            fx.area().map(|a| a.intersection(buf.area)).unwrap_or(Rect::default())
        } else {
            area
        };

        if let Some(content_fx) = self.content_fx.as_mut() {
            content_fx.set_area(area)
        }

        if area != Rect::default() {
            self.window_block().render(area, buf);
        }

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
    }


    fn done(&self) -> bool {
        self.pre_render_fx.is_none()
            || self.pre_render_fx.as_ref().is_some_and(Effect::done)
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.pre_render_fx.as_ref()
            .map(Effect::area)
            .unwrap_or(None)
    }

    fn set_area(&mut self, area: Rect) {
        if let Some(open_window_fx) = self.pre_render_fx.as_mut() {
            open_window_fx.set_area(area);
        }
    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {
        todo!()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        self.pre_render_fx.as_mut().and_then(Effect::timer_mut)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.pre_render_fx.as_ref().map(Effect::cell_selection).flatten()
    }

    fn reset(&mut self) {
        if let Some(fx) = self.pre_render_fx.as_mut() {
            fx.reset();
        }
    }
}