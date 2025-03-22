use crate::features::acquire_ref;
use crate::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, RefCount, Shader, ThreadSafetyMarker};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::fmt::Debug;

pub type InstanceId = u32;

#[derive(Clone, Debug)]
pub struct Unique<K: Clone + ThreadSafetyMarker> {
    id_context: RefCount<UniqueContext<K>>,
    instance_id: InstanceId,
    fx: Effect,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct UniqueContext<K: Clone + ThreadSafetyMarker> {
    pub key: K,
    pub instance_id: InstanceId,
}

impl<K: Clone + ThreadSafetyMarker> UniqueContext<K> {
    pub(crate) fn new(key: impl Into<K>, instance_id: InstanceId) -> Self {
        Self {
            key: key.into(),
            instance_id,
        }
    }
}

impl<K: Clone + ThreadSafetyMarker> Unique<K> {
    pub(crate) fn new(id_context: RefCount<UniqueContext<K>>, fx: Effect) -> Self {
        let instance_id = acquire_ref(&id_context).instance_id;
        Self {
            id_context,
            instance_id,
            fx,
        }
    }
}

impl<K: Clone + Debug + ThreadSafetyMarker + 'static> Shader for Unique<K> {
    fn name(&self) -> &'static str {
        "unique"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.fx.process(duration, buf, area)
    }

    fn done(&self) -> bool {
        let iid = acquire_ref(&self.id_context).instance_id;
        self.instance_id != iid || self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter);
    }

    fn reverse(&mut self) {
        self.fx.reverse();
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.fx.timer()
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        self.fx.cell_filter()
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn reset(&mut self) {
        self.fx.reset();
    }
}