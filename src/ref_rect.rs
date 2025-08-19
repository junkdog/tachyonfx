use ratatui::layout::{Position, Rect};

use crate::{ref_count, RefCount};

/// A reference-counted, mutable rectangle that can be shared between multiple effects.
///
/// `RefRect` provides a way to share a mutable `Rect` between multiple components
/// without requiring ownership transfer. This is particularly useful for effects
/// that need to operate on the same dynamically changing area.
///
/// # Examples
///
/// ```rust
/// use tachyonfx::RefRect;
/// use ratatui::layout::Rect;
///
/// let ref_rect = RefRect::new(Rect::new(0, 0, 10, 5));
///
/// // Multiple components can share the same area reference
/// let shared_rect = ref_rect.clone();
///
/// // Update the area from one component
/// ref_rect.set(Rect::new(5, 5, 20, 10));
///
/// // All sharing components see the update
/// assert_eq!(shared_rect.get(), Rect::new(5, 5, 20, 10));
/// ```
#[derive(Clone, Debug)]
pub struct RefRect {
    rect: RefCount<Rect>,
}

impl RefRect {
    /// Creates a new `RefRect` with the specified initial rectangle.
    ///
    /// # Arguments
    ///
    /// * `rect` - The initial rectangle value
    ///
    /// # Returns
    ///
    /// A new `RefRect` containing the specified rectangle
    pub fn new(rect: Rect) -> Self {
        Self { rect: ref_count(rect) }
    }

    /// Gets the current rectangle value.
    ///
    /// # Returns
    ///
    /// A copy of the current `Rect` value
    pub fn get(&self) -> Rect {
        #[cfg(feature = "sendable")]
        {
            *self.rect.lock().unwrap()
        }
        #[cfg(not(feature = "sendable"))]
        {
            *self.rect.borrow()
        }
    }

    /// Sets the rectangle to a new value.
    ///
    /// All clones of this `RefRect` will see the updated value.
    ///
    /// # Arguments
    ///
    /// * `rect` - The new rectangle value to set
    pub fn set(&self, rect: Rect) {
        #[cfg(feature = "sendable")]
        {
            *self.rect.lock().unwrap() = rect;
        }
        #[cfg(not(feature = "sendable"))]
        {
            *self.rect.borrow_mut() = rect;
        }
    }

    /// Checks if the rectangle contains the specified position.
    ///
    /// # Arguments
    ///
    /// * `position` - The position to check
    ///
    /// # Returns
    ///
    /// `true` if the position is within the rectangle bounds, `false` otherwise
    pub fn contains(&self, position: Position) -> bool {
        #[cfg(feature = "sendable")]
        {
            self.rect.lock().unwrap().contains(position)
        }
        #[cfg(not(feature = "sendable"))]
        {
            self.rect.borrow().contains(position)
        }
    }

    /// Returns the top y-coordinate of the rectangle.
    ///
    /// # Returns
    ///
    /// The y-coordinate of the top edge
    pub fn top(&self) -> u16 {
        #[cfg(feature = "sendable")]
        {
            self.rect.lock().unwrap().top()
        }
        #[cfg(not(feature = "sendable"))]
        {
            self.rect.borrow().top()
        }
    }

    /// Returns the bottom y-coordinate of the rectangle (outside the rect).
    ///
    /// This is equivalent to `y + height`.
    ///
    /// # Returns
    ///
    /// The y-coordinate of the bottom edge (exclusive)
    pub fn bottom(&self) -> u16 {
        #[cfg(feature = "sendable")]
        {
            self.rect.lock().unwrap().bottom()
        }
        #[cfg(not(feature = "sendable"))]
        {
            self.rect.borrow().bottom()
        }
    }

    /// Returns the left x-coordinate of the rectangle.
    ///
    /// # Returns
    ///
    /// The x-coordinate of the left edge
    pub fn left(&self) -> u16 {
        #[cfg(feature = "sendable")]
        {
            self.rect.lock().unwrap().left()
        }
        #[cfg(not(feature = "sendable"))]
        {
            self.rect.borrow().left()
        }
    }

    /// Returns the right x-coordinate of the rectangle (outside the rect).
    ///
    /// This is equivalent to `x + width`.
    ///
    /// # Returns
    ///
    /// The x-coordinate of the right edge (exclusive)
    pub fn right(&self) -> u16 {
        #[cfg(feature = "sendable")]
        {
            self.rect.lock().unwrap().right()
        }
        #[cfg(not(feature = "sendable"))]
        {
            self.rect.borrow().right()
        }
    }
}

impl Default for RefRect {
    fn default() -> Self {
        Self::new(Rect::default())
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::{Position, Rect};

    use super::*;

    #[test]
    fn test_ref_rect_new_and_get() {
        let rect = Rect::new(10, 20, 30, 40);
        let ref_rect = RefRect::new(rect);
        assert_eq!(ref_rect.get(), rect);
    }

    #[test]
    fn test_ref_rect_set() {
        let initial_rect = Rect::new(10, 20, 30, 40);
        let new_rect = Rect::new(5, 15, 25, 35);

        let ref_rect = RefRect::new(initial_rect);
        assert_eq!(ref_rect.get(), initial_rect);

        ref_rect.set(new_rect);
        assert_eq!(ref_rect.get(), new_rect);
    }

    #[test]
    fn test_ref_rect_clone_shares_state() {
        let initial_rect = Rect::new(10, 20, 30, 40);
        let new_rect = Rect::new(5, 15, 25, 35);

        let ref_rect1 = RefRect::new(initial_rect);
        let ref_rect2 = ref_rect1.clone();

        // Both should have the same initial value
        assert_eq!(ref_rect1.get(), initial_rect);
        assert_eq!(ref_rect2.get(), initial_rect);

        // Changing one should affect the other
        ref_rect1.set(new_rect);
        assert_eq!(ref_rect1.get(), new_rect);
        assert_eq!(ref_rect2.get(), new_rect);

        // And vice versa
        let third_rect = Rect::new(1, 2, 3, 4);
        ref_rect2.set(third_rect);
        assert_eq!(ref_rect1.get(), third_rect);
        assert_eq!(ref_rect2.get(), third_rect);
    }

    #[test]
    fn test_ref_rect_contains() {
        let rect = Rect::new(10, 20, 30, 40);
        let ref_rect = RefRect::new(rect);

        // Position inside the rect
        assert!(ref_rect.contains(Position::new(15, 25)));
        assert!(ref_rect.contains(Position::new(10, 20))); // top-left corner
        assert!(ref_rect.contains(Position::new(39, 59))); // bottom-right corner (exclusive)

        // Position outside the rect
        assert!(!ref_rect.contains(Position::new(5, 15)));
        assert!(!ref_rect.contains(Position::new(40, 60)));
        assert!(!ref_rect.contains(Position::new(15, 70)));
    }

    #[test]
    fn test_ref_rect_default() {
        let ref_rect = RefRect::default();
        assert_eq!(ref_rect.get(), Rect::default());
    }

    #[test]
    fn test_ref_rect_edge_methods() {
        let rect = Rect::new(10, 20, 30, 40);
        let ref_rect = RefRect::new(rect);

        assert_eq!(ref_rect.top(), 20);
        assert_eq!(ref_rect.bottom(), 60); // 20 + 40
        assert_eq!(ref_rect.left(), 10);
        assert_eq!(ref_rect.right(), 40); // 10 + 30
    }
}
