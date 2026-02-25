const MAX_CACHE_SIZE: usize = 255; // Limited by u8 index space

/// A fixed-size LRU (Least Recently Used) cache with const generic capacity.
/// This cache is designed for high-performance scenarios with small cache sizes.
///
/// This cache stores key-value pairs and automatically evicts the least recently
/// used item when the capacity is reached. It also tracks cache hit/miss statistics.
///
/// The implementation is optimized for small cache sizes (typically 8 entries)
/// and uses stack-allocated arrays for maximum performance. The LRU ordering is
/// maintained using efficient array operations without heap allocations. More
/// recently accessed entries are moved to the front of the index array.
///
/// # Type Parameters
///
/// * `K` - The key type, must implement `PartialEq + Copy`
/// * `V` - The value type
/// * `N` - The fixed capacity of the cache (must be between 1 and 255)
///
/// # Examples
///
/// ```
/// use tachyonfx::LruCache;
///
/// // Small cache size for optimal performance
/// let mut cache = LruCache::<i32, String, 8>::new();
///
/// // The memoize method computes a value if not in cache
/// let value = cache.memoize(&42, |k| format!("value_{}", k));
/// assert_eq!(value, "value_42");
///
/// // Second access is a cache hit
/// let value = cache.memoize(&42, |_| panic!("Should not be called"));
/// assert_eq!(value, "value_42");
///
/// // When capacity is reached, least recently used item is evicted
/// for i in 1..=8 {
///     cache.memoize(&i, |k| format!("value_{}", k));
/// }
/// cache.memoize(&999, |k| format!("value_{}", k)); // Evicts LRU item
/// ```
#[derive(Debug)]
pub struct LruCache<K, V, const N: usize>
where
    K: PartialEq + Copy,
{
    index: [(K, u8); N],
    entries: [V; N],
    cache_misses: u32,
    cache_hits: u32,
}

impl<K, V, const N: usize> LruCache<K, V, N>
where
    K: PartialEq + Copy,
{
    const _VALIDATE_SIZE: () = assert!(
        N > 0 && N <= MAX_CACHE_SIZE,
        "Cache size must be between 1 and 255"
    );

    /// Creates a new empty LRU cache with the specified capacity.
    ///
    /// # Panics
    ///
    /// Panics if `N` is 0 or greater than 255.
    pub fn new() -> Self
    where
        K: Default,
        V: Default,
    {
        // force evaluation of the const assertion
        #[allow(clippy::let_unit_value)]
        let _ = Self::_VALIDATE_SIZE;

        Self {
            index: core::array::from_fn(|i| (K::default(), i as u8)),
            entries: core::array::from_fn(|_| V::default()),
            cache_misses: 0,
            cache_hits: 0,
        }
    }

    /// Retrieves a value from the cache, or computes and caches it using the provided
    /// function. Note that this method returns a clone of the value.
    ///
    /// If the key exists in the cache, its value is returned and marked as recently used.
    /// If the key doesn't exist, the function `f` is called to compute the value, which
    /// is then stored in the cache before being returned.
    ///
    /// When the cache is full, the least recently used entry is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    /// * `f` - Function to compute the value if the key is not in the cache
    ///
    /// # Returns
    ///
    /// The value associated with the key, either from the cache or newly computed
    pub fn memoize(&mut self, key: &K, f: impl FnOnce(&K) -> V) -> V
    where
        V: Clone,
    {
        self.memoize_ref(key, f).clone()
    }

    /// Retrieves a reference from the cache, or computes and caches it using the provided
    /// function.
    ///
    /// If the key exists in the cache, its value is returned and marked as recently used.
    /// If the key doesn't exist, the function `f` is called to compute the value, which
    /// is then stored in the cache before being returned.
    ///
    /// When the cache is full, the least recently used entry is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    /// * `f` - Function to compute the value if the key is not in the cache
    ///
    /// # Returns
    ///
    /// The value associated with the key, either from the cache or newly computed
    pub fn memoize_ref(&mut self, key: &K, f: impl FnOnce(&K) -> V) -> &V {
        match self.refresh_key(key) {
            RefreshResult::Hit(value_idx) => &self.entries[value_idx],
            RefreshResult::Miss(value_idx) => {
                self.entries[value_idx] = f(key);
                &self.entries[value_idx]
            },
        }
    }

    /// Returns the number of cache hits since creation.
    pub fn cache_hits(&self) -> u32 {
        self.cache_hits
    }

    /// Returns the number of cache misses since creation.
    pub fn cache_misses(&self) -> u32 {
        self.cache_misses
    }

    fn refresh_key(&mut self, key: &K) -> RefreshResult {
        if let Some((idx, entry_idx)) = self
            .index
            .iter()
            .enumerate()
            .find(|&(_, v)| v.0 == *key)
            .map(|(idx, (_key, entry_idx))| (idx, *entry_idx))
        {
            self.cache_hits += 1;

            if idx > 0 {
                let index_record = self.index[idx];
                self.index.copy_within(0..idx, 1);
                self.index[0] = index_record;
            }

            RefreshResult::Hit(entry_idx as usize)
        } else {
            self.cache_misses += 1;

            let entry_idx = self.index[N - 1].1;
            self.index.copy_within(0..N - 1, 1);
            self.index[0] = (*key, entry_idx);

            RefreshResult::Miss(entry_idx as usize)
        }
    }
}

enum RefreshResult {
    Hit(usize),
    Miss(usize),
}

impl<K, V, const N: usize> Default for LruCache<K, V, N>
where
    K: PartialEq + Copy + Default,
    V: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, const N: usize> Clone for LruCache<K, V, N>
where
    K: Copy + PartialEq,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            entries: self.entries.clone(),
            cache_misses: 0,
            cache_hits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use super::*;

    #[test]
    fn test_memoize_adds_entry() {
        let mut cache: LruCache<&str, i32, 5> = LruCache::new();

        let compute_called = core::cell::Cell::new(0);
        let result = cache.memoize(&"key1", |_| {
            compute_called.set(compute_called.get() + 1);
            42
        });

        assert_eq!(result, 42);
        assert_eq!(compute_called.get(), 1);
    }

    #[test]
    fn test_cache_hit_reuses_value() {
        let mut cache: LruCache<&str, i32, 5> = LruCache::new();
        let compute_count = core::cell::Cell::new(0);

        // First call computes the value
        let val1 = cache.memoize(&"key1", |_| {
            compute_count.set(compute_count.get() + 1);
            100
        });

        // Second call should reuse the cached value
        let val2 = cache.memoize(&"key1", |_| {
            compute_count.set(compute_count.get() + 1);
            999 // Different value to verify it's not recomputed
        });

        assert_eq!(val1, 100);
        assert_eq!(val2, 100); // Should return the first computed value
        assert_eq!(compute_count.get(), 1); // Function should only be called once
    }

    #[test]
    fn test_capacity_limit_enforced() {
        let mut cache: LruCache<i32, i32, 3> = LruCache::new();

        cache.memoize(&1, |k| k * 10);
        cache.memoize(&2, |k| k * 10);
        cache.memoize(&3, |k| k * 10);

        // Cache should now be full
        // Adding a new item should evict the least recently used (1)
        cache.memoize(&4, |k| k * 10);

        // Checking if key 1 is recomputed to verify it was evicted
        let computation_occurred = core::cell::Cell::new(false);
        cache.memoize(&1, |k| {
            computation_occurred.set(true);
            k * 10
        });

        assert!(computation_occurred.get(), "Key 1 should have been evicted");
    }

    #[test]
    fn test_lru_eviction_policy() {
        let mut cache: LruCache<&str, i32, 3> = LruCache::new();

        // Add initial items
        cache.memoize(&"a", |_| 1);
        cache.memoize(&"b", |_| 2);
        cache.memoize(&"c", |_| 3);

        // Access "a" to make it most recently used
        cache.memoize(&"a", |_| 1);

        // Add new item that should evict "b" (now the LRU)
        cache.memoize(&"d", |_| 4);

        // Verify "b" was evicted
        let mut recompute_counter = 0;
        cache.memoize(&"b", |_| {
            recompute_counter += 1;
            2
        });

        assert_eq!(recompute_counter, 1, "Key 'b' should have been evicted");

        // Verify "a", "c", and "d" are still in the cache
        let mut compute_count = 0;
        cache.memoize(&"a", |_| {
            compute_count += 1;
            1
        });
        cache.memoize(&"b", |_| {
            compute_count += 1;
            2
        });
        cache.memoize(&"d", |_| {
            compute_count += 1;
            4
        });

        assert_eq!(
            compute_count, 0,
            "Keys 'a', 'b', and 'd' should still be cached"
        );
    }

    #[test]
    fn test_complex_key_types() {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        struct ComplexKey {
            id: &'static str,
            section: u32,
        }

        let mut cache: LruCache<ComplexKey, Vec<i32>, 3> = LruCache::new();

        let key1 = ComplexKey { id: "test", section: 1 };
        let key2 = ComplexKey { id: "test", section: 2 };

        cache.memoize(&key1, |_| vec![1, 2, 3]);
        cache.memoize(&key2, |_| vec![4, 5, 6]);

        // Retrieve with the same key structure
        let result = cache.memoize(&key1, |_| vec![99, 99, 99]);

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_large_values_handling() {
        let mut cache: LruCache<i32, Vec<u8>, 2> = LruCache::new();

        // Create a large value
        let large_value = vec![0u8; 1024];

        cache.memoize(&1, |_| large_value.clone());

        // Check if large value is cached correctly
        let retrieved = cache.memoize(&1, |_| vec![1u8; 1024]);
        assert_eq!(retrieved, large_value);
    }

    #[test]
    fn test_cache_hit_and_miss_statistics() {
        // This test would work if we added the stats method suggested in the feedback
        let mut cache: LruCache<&str, i32, 3> = LruCache::new();

        // Initial misses
        cache.memoize(&"a", |_| 1);
        cache.memoize(&"b", |_| 2);

        // Hits
        cache.memoize(&"a", |_| 999);
        cache.memoize(&"b", |_| 999);
        cache.memoize(&"a", |_| 999);

        let (hits, misses) = (cache.cache_hits(), cache.cache_misses());
        assert_eq!(hits, 3);
        assert_eq!(misses, 2);
    }

    #[test]
    fn test_entry_index_correctness() {
        // This test ensures that cache misses get the correct entry index
        // and don't cause misidentifications
        let mut cache: LruCache<i32, String, 3> = LruCache::new();

        // Fill cache completely
        cache.memoize(&1, |k| format!("value{k}"));
        cache.memoize(&2, |k| format!("value{k}"));
        cache.memoize(&3, |k| format!("value{k}"));

        // All should be hits and return correct values
        assert_eq!(cache.memoize(&1, |_| "wrong".to_string()), "value1");
        assert_eq!(cache.memoize(&2, |_| "wrong".to_string()), "value2");
        assert_eq!(cache.memoize(&3, |_| "wrong".to_string()), "value3");

        // Add new item, should evict one
        cache.memoize(&4, |k| format!("value{k}"));

        // New item should be cached correctly
        assert_eq!(cache.memoize(&4, |_| "wrong".to_string()), "value4");
    }
}
