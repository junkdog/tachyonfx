//! tachyonfx - A ratatui library for creating shader-like effects in terminal UIs
//!
//! This library provides a collection of effects that can be used to enhance the visual
//! appeal of terminal applications, offering capabilities such as color transformations,
//! animations, and complex effect combinations.

mod bounding_box;
mod buffer_renderer;
mod cell_filter;
mod cell_iter;
mod color_cache;
mod color_ext;
mod color_mapper;
mod color_space;
mod duration;
mod effect;
mod effect_manager;
mod effect_timer;
mod features;
mod interpolation;
mod lru_cache;
mod motion;
mod rect_ext;
mod ref_rect;
mod render_effect;
mod shader;
mod simple_rng;

pub mod fx;
pub mod widget;

#[cfg(feature = "dsl")]
#[doc = include_str!("../docs/dsl.md")]
pub mod dsl;

pub use buffer_renderer::*;
pub use cell_filter::*;
/// `CellIterator` provides an iterator over terminal cells.
pub use cell_iter::CellIterator;
pub use color_cache::ColorCache;
pub use color_ext::ToRgbComponents;
#[allow(deprecated)]
pub use color_mapper::ColorMapper;
pub use color_space::*;
pub use duration::Duration;
#[allow(unused_imports)] // not actually unused, misidentified by clippy
pub(crate) use effect::ShaderExt;
pub use effect::{Effect, IntoEffect};
pub use effect_manager::EffectManager;
pub use effect_timer::EffectTimer;
pub use features::{ref_count, RefCount, ThreadSafetyMarker};
pub use interpolation::*;
pub use lru_cache::LruCache;
pub use motion::*;
pub use rect_ext::CenteredShrink;
pub use ref_rect::RefRect;
pub use render_effect::EffectRenderer;
pub use shader::Shader;
pub use simple_rng::*;

extern crate alloc;

#[cfg(all(feature = "std-duration", feature = "web-time"))]
compile_error!("Features 'std-duration' and 'web-time' cannot be enabled simultaneously");
