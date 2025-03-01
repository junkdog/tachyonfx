pub use sendable::ThreadSafetyMarker;
pub use sendable::RefCount;
use crate::fx::unique::UniqueContext;


#[cfg(feature = "sendable")]
mod sendable {
    use std::sync::{Arc, Mutex};

    pub trait ThreadSafetyMarker : Send {}
    impl<T: Send> ThreadSafetyMarker for T {}

    pub type RefCount<T> = Arc<Mutex<T>>;

    pub fn ref_count<T>(value: T) -> RefCount<T> {
        Arc::new(Mutex::new(value))
    }
}

#[cfg(not(feature = "sendable"))]
mod sendable {
    use std::cell::RefCell;
    use std::rc::Rc;

    pub trait ThreadSafetyMarker {}
    impl<T> ThreadSafetyMarker for T {}

    pub type RefCount<T> = Rc<RefCell<T>>;

    pub fn ref_count<T>(value: T) -> RefCount<T> {
        Rc::new(RefCell::new(value))
    }
}


#[cfg(feature = "sendable")]
pub(crate) fn acquire_mut<K: Clone + ThreadSafetyMarker>(
    ctx: &RefCount<UniqueContext<K>>,
) -> std::sync::MutexGuard<'_, UniqueContext<K>> {
    ctx.lock().unwrap()
}

#[cfg(feature = "sendable")]
pub(crate) fn acquire_ref<K: Clone + ThreadSafetyMarker>(
    ctx: &RefCount<UniqueContext<K>>,
) -> std::sync::MutexGuard<'_, UniqueContext<K>> {
    ctx.lock().unwrap()
}

#[cfg(not(feature = "sendable"))]
pub(crate) fn acquire_mut<K: Clone + ThreadSafetyMarker>(
    ctx: &RefCount<UniqueContext<K>>,
) -> std::cell::RefMut<'_, UniqueContext<K>> {
    ctx.borrow_mut()
}

#[cfg(not(feature = "sendable"))]
pub(crate) fn acquire_ref<K: Clone + ThreadSafetyMarker>(
    ctx: &RefCount<UniqueContext<K>>,
) -> std::cell::Ref<'_, UniqueContext<K>> {
    ctx.borrow()
}

/// Wraps a value in a reference-counted smart pointer.
///
/// This function creates a reference-counted wrapper around the provided value. The exact type
/// of the wrapper depends on the "sendable" feature flag:
///
/// - When the "sendable" feature is enabled, it returns an `Arc<Mutex<T>>`.
/// - When the "sendable" feature is disabled, it returns an `Rc<RefCell<T>>`.
///
/// # Arguments
///
/// * `value`: The value to be wrapped.
///
/// # Returns
///
/// Returns a `RefCount<T>`, which is an alias for either `Arc<Mutex<T>>` or `Rc<RefCell<T>>`,
/// depending on the "sendable" feature flag.
///
/// # Examples
///
/// ```
/// use tachyonfx::ref_count;
///
/// let wrapped = ref_count(42);
/// ```
///
/// # Feature Flags
///
/// - When the "sendable" feature is enabled, this function produces thread-safe wrappers.
/// - When the "sendable" feature is disabled, this function produces non-thread-safe wrappers
///   that are more efficient in single-threaded contexts.
pub fn ref_count<T>(value: T) -> RefCount<T> {
    sendable::ref_count(value)
}
