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

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub use cell_iter::CellIterator;
pub use color_mapper::ColorMapper;
pub use effect::{Effect, CellFilter, IntoEffect};
pub use effect_timer::EffectTimer;
pub use rect_ext::CenteredShrink;
pub use render_effect::EffectRenderer;
pub use shader::Shader;
pub use interpolation::*;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
