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
mod motion;

pub mod fx;
pub mod widget;
mod bounding_box;
mod buffer_renderer;
mod cell_filter;
mod simple_rng;
mod duration;
mod features;

#[cfg(feature = "dsl")]
pub mod dsl;

/// `CellIterator` provides an iterator over terminal cells.
pub use cell_iter::CellIterator;
pub use color_mapper::ColorMapper;
pub use cell_filter::{CellFilter, CellPredicate};
pub use effect::{Effect, IntoEffect};
pub use effect_timer::EffectTimer;
pub use rect_ext::CenteredShrink;
pub use render_effect::EffectRenderer;
pub use shader::Shader;
pub use interpolation::*;
pub use buffer_renderer::*;
pub use simple_rng::*;
pub use duration::Duration;
pub use motion::*;
pub use features::{ref_count, RefCount, ThreadSafetyMarker};

#[cfg(all(feature = "std-duration", feature = "web-time"))]
compile_error!("Features 'std-duration' and 'web-time' cannot be enabled simultaneously");