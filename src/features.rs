pub use sendable::{RefCount, Shared, ThreadSafetyMarker};

use crate::fx::unique::UniqueContext;

#[cfg(all(feature = "sendable", not(feature = "std")))]
compile_error!("Feature 'sendable' requires 'std' feature for thread synchronization primitives");

#[cfg(all(feature = "sendable", feature = "std"))]
mod sendable {
    use alloc::sync::Arc;
    use std::sync::Mutex;

    pub trait ThreadSafetyMarker: Send {}
    impl<T: Send> ThreadSafetyMarker for T {}

    pub type RefCount<T> = Arc<Mutex<T>>;
    pub type Shared<T> = Arc<T>;

    pub fn ref_count<T>(value: T) -> RefCount<T> {
        Arc::new(Mutex::new(value))
    }
}

#[cfg(not(feature = "sendable"))]
mod sendable {
    use alloc::rc::Rc;
    use core::cell::RefCell;

    pub trait ThreadSafetyMarker {}
    impl<T> ThreadSafetyMarker for T {}

    pub type RefCount<T> = Rc<RefCell<T>>;
    pub type Shared<T> = Rc<T>;

    pub fn ref_count<T>(value: T) -> RefCount<T> {
        Rc::new(RefCell::new(value))
    }
}

#[cfg(all(feature = "sendable", feature = "std"))]
pub(crate) fn acquire_mut(
    ctx: &RefCount<UniqueContext>,
) -> std::sync::MutexGuard<'_, UniqueContext> {
    ctx.lock().unwrap()
}

#[cfg(all(feature = "sendable", feature = "std"))]
pub(crate) fn acquire_ref(
    ctx: &RefCount<UniqueContext>,
) -> std::sync::MutexGuard<'_, UniqueContext> {
    ctx.lock().unwrap()
}

#[cfg(not(feature = "sendable"))]
pub(crate) fn acquire_mut(ctx: &RefCount<UniqueContext>) -> core::cell::RefMut<'_, UniqueContext> {
    ctx.borrow_mut()
}

#[cfg(not(feature = "sendable"))]
pub(crate) fn acquire_ref(ctx: &RefCount<UniqueContext>) -> core::cell::Ref<'_, UniqueContext> {
    ctx.borrow()
}

/// Wraps a value in a reference-counted smart pointer.
///
/// This function creates a reference-counted wrapper around the provided value. The exact
/// type of the wrapper depends on the "sendable" feature flag:
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
/// Returns a `RefCount<T>`, which is an alias for either `Arc<Mutex<T>>` or
/// `Rc<RefCell<T>>`, depending on the "sendable" feature flag.
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
/// - When the "sendable" feature is disabled, this function produces non-thread-safe
///   wrappers that are more efficient in single-threaded contexts.
pub fn ref_count<T>(value: T) -> RefCount<T> {
    sendable::ref_count(value)
}
