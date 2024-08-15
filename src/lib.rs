//! tachyonfx - A ratatui library for creating shader-like effects in terminal UIs
//!
//! This library provides a collection of effects that can be used to enhance the visual
//! appeal of terminal applications, offering capabilities such as color transformations,
//! animations, and complex effect combinations.

mod interpolation;
mod effect;
mod shader;
mod effect_timer;
mod cell_iter;
mod color_mapper;
mod color_ext;
mod rect_ext;
mod render_effect;

pub mod fx;
pub mod widget;
mod bounding_box;
mod buffer_renderer;
mod cell_filter;


/// `CellIterator` provides an iterator over terminal cells.
pub use cell_iter::CellIterator;
pub use color_mapper::ColorMapper;
pub use cell_filter::CellFilter;
pub use effect::{Effect, IntoEffect};
pub use effect_timer::EffectTimer;
pub use rect_ext::CenteredShrink;
pub use render_effect::EffectRenderer;
pub use shader::Shader;
pub use interpolation::*;
pub use buffer_renderer::{BufferRenderer, blit_buffer, render_as_ansi_string};

