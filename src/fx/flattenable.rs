use crate::Effect;

pub trait FlattenableEffect {
    fn flatten(&self) -> Vec<Effect>;
}
