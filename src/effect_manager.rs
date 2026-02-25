use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt::Debug;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    features::acquire_mut,
    fx,
    fx::unique::{Unique, UniqueContext},
    ref_count, Duration, Effect, IntoEffect, RefCount, SimpleRng, ThreadSafetyMarker,
};

/// Manages a collection of terminal UI effects, including uniquely identified
/// effects that can be replaced/cancelled by new effects with the same id.
///
/// The `EffectManager` provides lifecycle management for both regular effects and unique
/// effects. Regular effects run until completion, while unique effects can be cancelled
/// when a new effect with the same identifier is added.
#[derive(Debug, Default)]
pub struct EffectManager<K: Clone + Ord + ThreadSafetyMarker + 'static> {
    effects: Vec<Effect>,
    uniques: BTreeMap<K, RefCount<UniqueContext>>,
    rng: SimpleRng,
}

#[allow(dead_code)]
impl<K: Clone + Debug + Ord + ThreadSafetyMarker> EffectManager<K> {
    /// Creates a unique effect that will cancel any existing effect with the same key.
    /// The effect must be added to the manager using [`Self::add_effect`] in order to be
    /// processed.
    ///
    /// When a new unique effect is created with a key that matches an existing effect,
    /// the existing effect will be marked as complete on the next processing cycle.
    ///
    /// # Arguments
    /// * `key` - A unique identifier for the effect. If an effect with this key already
    ///   exists, the existing effect will be cancelled.
    /// * `fx` - The effect to be wrapped with unique identification.
    ///
    /// # Returns
    /// A new effect that includes unique identification logic. The effect must still be
    /// added to the manager to be processed.
    pub fn unique(&mut self, key: impl Into<K>, fx: Effect) -> Effect {
        let key = key.into();
        let ctx = self
            .uniques
            .entry(key)
            .and_modify(|ctx| acquire_mut(ctx).instance_id = self.rng.gen())
            .or_insert_with(|| ref_count(UniqueContext::new(self.rng.gen())))
            .clone();

        Unique::new(ctx, fx).into_effect()
    }

    /// Adds an effect to be processed by the manager.
    ///
    /// The effect will be processed each frame until it is complete.
    ///
    /// # Arguments
    /// * `effect` - The effect to add to the manager
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Cancels a unique effect by its key.
    ///
    /// This method stops a running unique effect by replacing it with a zero-duration
    /// effect that completes immediately. The cancelled effect will be removed on the
    /// next call to [`Self::process_effects`].
    ///
    /// Note: This only works for effects added via [`Self::add_unique_effect`] or
    /// [`Self::unique`]. Regular effects added through [`Self::add_effect`] cannot
    /// be stopped this way.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the effect to cancel
    pub fn cancel_unique_effect(&mut self, id: impl Into<K>) {
        self.add_unique_effect(id, fx::consume_tick());
    }

    /// Creates and adds a unique effect to the manager in a single operation.
    ///
    /// This is a convenience method that combines [`Self::unique`] and
    /// [`Self::add_effect`]. Any existing effect with the same key will be cancelled.
    ///
    /// # Arguments
    /// * `key` - A unique identifier for the effect. If an effect with this key already
    ///   exists, the existing effect will be cancelled.
    /// * `fx` - The effect to be wrapped with unique identification and added to the
    ///   manager.
    pub fn add_unique_effect(&mut self, key: impl Into<K>, fx: Effect) {
        let fx = self.unique(key, fx);
        self.add_effect(fx);
    }

    /// Returns whether there are any active effects currently being managed.
    ///
    /// This method is useful for optimizing render loops: if `is_running()` returns
    /// `false` and there are no other state changes, the UI redraw can be skipped.
    ///
    /// # Returns
    /// `true` if there are active effects, `false` otherwise.
    ///
    /// # Example
    /// ```ignore
    /// if effect_manager.is_running() || other_state_changed {
    ///     terminal.draw(|frame| {
    ///         // render UI
    ///     })?;
    /// }
    /// ```
    pub fn is_running(&self) -> bool {
        !self.effects.is_empty()
    }

    /// Processes all active effects for the given duration.
    ///
    /// This method should be called each frame in your render loop. It will:
    /// 1. Process each effect for the specified duration
    /// 2. Remove completed effects
    /// 3. Clean up any orphaned unique effect contexts
    ///
    /// # Arguments
    /// * `duration` - The time elapsed since the last frame
    /// * `buf` - The buffer to render effects into
    /// * `area` - The area within which effects should be rendered
    pub fn process_effects(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) {
        self.effects.retain_mut(|effect| {
            effect.process(duration, buf, area);
            effect.running()
        });

        // clear orphaned unique effects;
        self.uniques
            .retain(|_, ctx| RefCount::strong_count(ctx) > 1);
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        boxed::Box,
        string::{String, ToString},
    };
    use core::fmt::Debug;

    use ratatui_core::{buffer::Buffer, layout::Rect};

    use super::*;
    use crate::{CellFilter, Shader};

    #[test]
    fn test_process_effects_removes_completed() {
        let mut manager = EffectManager::<String>::default();

        // add an effect that completes after 1 process
        let effect1 = counter_effect(1);

        // add an effect that completes after 3 processes
        let effect2 = counter_effect(3);

        manager.add_effect(effect1);
        manager.add_effect(effect2);

        assert_eq!(manager.effects.len(), 2);

        // process once - should remove the first effect
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        assert_eq!(manager.effects.len(), 1);

        // process twice more - should remove the second effect
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        assert_eq!(manager.effects.len(), 0);
    }

    #[test]
    fn test_unique_effects_same_key_cancels_previous() {
        let mut manager = EffectManager::<&'static str>::default();

        // Create a unique effect with key "test" that would complete after 5 processes
        let effect1 = manager.unique("test", counter_effect(5));
        manager.add_effect(effect1);

        // process once
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        assert_eq!(manager.effects.len(), 1);

        // add another effect with the same key
        let effect2 = manager.unique("test", counter_effect(5));
        manager.add_effect(effect2);

        // process again - the first effect should be cancelled and removed
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        // only the second effect should remain
        assert_eq!(manager.effects.len(), 1);
    }

    #[test]
    fn test_add_unique_effect_convenience_method() {
        let mut manager = EffectManager::<&'static str>::default();

        // add a unique effect using the convenience method
        manager.add_unique_effect("test", counter_effect(3));

        assert_eq!(manager.effects.len(), 1);

        // add another unique effect with the same key
        manager.add_unique_effect("test", counter_effect(3));

        // both effects are in the list until processed
        assert_eq!(manager.effects.len(), 2);

        // process once - the first effect should be cancelled
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        // only the second effect should remain
        assert_eq!(manager.effects.len(), 1);
    }

    #[test]
    fn test_cleanup_orphaned_unique_contexts() {
        let mut manager = EffectManager::<&'static str>::default();

        // add and then process a unique effect until completion
        manager.add_unique_effect("test", counter_effect(1));

        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        // the effect is completed and removed
        assert_eq!(manager.effects.len(), 0);

        // the unique context should be cleaned up
        assert_eq!(manager.uniques.len(), 0);
    }

    #[test]
    fn test_different_keys_dont_interfere() {
        let mut manager = EffectManager::<String>::default();

        // add two unique effects with different keys
        manager.add_unique_effect("key1".to_string(), counter_effect(3));
        manager.add_unique_effect("key2".to_string(), counter_effect(3));

        assert_eq!(manager.effects.len(), 2);

        // add a new effect with key1
        manager.add_unique_effect("key1".to_string(), counter_effect(3));

        // now we have 3 effects
        assert_eq!(manager.effects.len(), 3);

        // process - the first key1 effect should be cancelled, but key2 should remain
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        manager.process_effects(
            Duration::from_millis(10),
            &mut buffer,
            Rect::new(0, 0, 10, 10),
        );

        // we should have 2 effects: the key2 effect and the new key1 effect
        assert_eq!(manager.effects.len(), 2);
    }

    // A simple test shader that just counts how many times it's been processed
    #[derive(Debug, Clone, Default)]
    struct CounterShader {
        count: usize,
        done_after: usize,
    }

    impl CounterShader {
        fn new(done_after: usize) -> Self {
            Self { count: 0, done_after }
        }
    }

    impl Shader for CounterShader {
        fn name(&self) -> &'static str {
            "counter"
        }

        fn process(
            &mut self,
            _duration: Duration,
            _buf: &mut Buffer,
            _area: Rect,
        ) -> Option<Duration> {
            self.count += 1;
            None
        }

        fn done(&self) -> bool {
            self.count >= self.done_after
        }
        fn clone_box(&self) -> Box<dyn Shader> {
            Box::new(self.clone())
        }
        fn area(&self) -> Option<Rect> {
            None
        }
        fn set_area(&mut self, _area: Rect) {}
        fn filter(&mut self, _filter: CellFilter) {}
    }

    fn counter_effect(done_after: usize) -> Effect {
        Effect::new(CounterShader::new(done_after))
    }

    #[test]
    #[cfg(not(feature = "sendable"))]
    fn test_effect_manager_with_zero_duration_shader_fn() {
        use alloc::rc::Rc;
        use core::cell::RefCell;

        use crate::fx;

        let mut manager = EffectManager::<String>::default();
        let call_count = Rc::new(RefCell::new(0));
        let call_count_clone = call_count.clone();

        // Create an effect with zero duration using fx::effect_fn
        let effect = fx::effect_fn((), Duration::ZERO, move |_state, _ctx, _cells| {
            *call_count_clone.borrow_mut() += 1;
        });

        manager.add_effect(effect);
        assert_eq!(manager.effects.len(), 1);

        // Process the effect once
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        let area = Rect::new(0, 0, 10, 10);
        manager.process_effects(Duration::from_millis(1), &mut buffer, area);

        // The effect should have been called at least once and then completed/removed
        assert_eq!(
            *call_count.borrow(),
            1,
            "Effect should have been called once"
        );
        assert_eq!(
            manager.effects.len(),
            0,
            "Effect should be removed after completion"
        );
    }

    #[test]
    #[cfg(not(feature = "sendable"))]
    fn test_effect_manager_with_normal_duration_shader_fn() {
        use alloc::rc::Rc;
        use core::cell::RefCell;

        use crate::fx;

        let mut manager = EffectManager::<String>::default();
        let call_count = Rc::new(RefCell::new(0));
        let call_count_clone = call_count.clone();

        // Create an effect with 100ms duration
        let effect = fx::effect_fn(
            (),
            Duration::from_millis(100),
            move |_state, _ctx, _cells| {
                *call_count_clone.borrow_mut() += 1;
            },
        );

        manager.add_effect(effect);
        assert_eq!(manager.effects.len(), 1);

        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        let area = Rect::new(0, 0, 10, 10);

        // Process for 50ms - effect should still be running
        manager.process_effects(Duration::from_millis(50), &mut buffer, area);
        assert_eq!(*call_count.borrow(), 1);
        assert_eq!(manager.effects.len(), 1, "Effect should still be running");

        // Process for another 60ms (total 110ms) - effect should complete
        manager.process_effects(Duration::from_millis(60), &mut buffer, area);
        assert_eq!(*call_count.borrow(), 2);
        assert_eq!(
            manager.effects.len(),
            0,
            "Effect should be completed and removed"
        );
    }

    #[test]
    #[cfg(not(feature = "sendable"))]
    fn test_multiple_zero_duration_shader_fn_effects() {
        use alloc::rc::Rc;
        use core::cell::RefCell;

        use crate::fx;

        let mut manager = EffectManager::<String>::default();
        let call_count = Rc::new(RefCell::new(0));

        // Add multiple zero-duration effects
        for i in 0..3 {
            let call_count_clone = call_count.clone();
            let effect = fx::effect_fn(i, Duration::ZERO, move |state, _ctx, _cells| {
                *call_count_clone.borrow_mut() += *state + 1;
            });
            manager.add_effect(effect);
        }

        assert_eq!(manager.effects.len(), 3);

        // Process once - all effects should run and be removed
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 10));
        let area = Rect::new(0, 0, 10, 10);
        manager.process_effects(Duration::from_millis(1), &mut buffer, area);

        // Total call count should be 0+1 + 1+1 + 2+1 = 6
        assert_eq!(
            *call_count.borrow(),
            6,
            "All effects should have been called"
        );
        assert_eq!(
            manager.effects.len(),
            0,
            "All effects should be completed and removed"
        );
    }
}
