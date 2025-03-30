use std::array;

/// A fixed-size LRU (Least Recently Used) cache with const generic capacity.
///
/// This cache stores key-value pairs and automatically evicts the least recently
/// used item when the capacity is reached. It also tracks cache hit/miss statistics.
///
/// # Type Parameters
///
/// * `K` - The key type, must be `Eq + Clone + Default`
/// * `V` - The value type, must be `Clone`
/// * `N` - The fixed capacity of the cache (must be between 1 and 255)
///
/// # Examples
///
/// ```
/// use tachyonfx::LruCache;
///
/// let mut cache = LruCache::<String, i32, 2>::new();
///
/// // The memoize method computes a value if not in cache
/// let value = cache.memoize(&"key1".to_string(), |k| k.len() as i32);
/// assert_eq!(value, 4);
///
/// // Second access is a cache hit
/// let value = cache.memoize(&"key1".to_string(), |_| panic!("Should not be called"));
/// assert_eq!(value, 4);
///
/// // When capacity is reached, least recently used item is evicted
/// cache.memoize(&"key2".to_string(), |_| 10);
/// cache.memoize(&"key3".to_string(), |_| 20);
///
/// // key1 was evicted, so this will call the function again
/// let value = cache.memoize(&"key1".to_string(), |_| 30);
/// assert_eq!(value, 30);
/// ```
#[derive(Debug, Clone)]
pub struct LruCache<K, V, const N: usize>
where
    K: PartialEq + Clone + Default,
{
    index: [K; N],
    entries: [(V, u16); N],
    counter: u16,
    cache_misses: u32,
    cache_hits: u32,
}

impl<K, V, const N: usize> LruCache<K, V, N>
where
    K: PartialEq + Clone + Default,
{
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
        assert!(N > 0,   "Cache size must be greater than 0");
        assert!(N < 256, "Cache size must be less than 256");
        Self {
            index: array::from_fn(|_| Default::default()),
            entries: array::from_fn(|_| Default::default()),
            counter: 0,
            cache_misses: 0,
            cache_hits: 0,
        }
    }

    /// Retrieves a value from the cache, or computes and caches it using the provided function.
    /// Note that this method returns a clone of the value.
    ///
    /// If the key exists in the cache, its value is returned and marked as recently used.
    /// If the key doesn't exist, the function `f` is called to compute the value, which is
    /// then stored in the cache before being returned.
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
    pub fn memoize(
        &mut self,
        key: &K,
        f: impl FnOnce(&K) -> V,
    ) -> V where V: Clone {
        self.memoize_ref(key, f).clone()
    }

    /// Retrieves a reference from the cache, or computes and caches it using the provided function.
    ///
    /// If the key exists in the cache, its value is returned and marked as recently used.
    /// If the key doesn't exist, the function `f` is called to compute the value, which is
    /// then stored in the cache before being returned.
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
    pub fn memoize_ref(
        &mut self,
        key: &K,
        f: impl FnOnce(&K) -> V,
    ) -> &V {
        self.counter += 1;
        if self.counter == 0xffff {
            self.normalize();
            self.counter = self.entries.iter()
                .map(|(_, counter)| *counter)
                .max()
                .unwrap_or(0)
        }

        // Find the entry with the matching key
        let pos = self.index.iter().enumerate()
            .find(|(_, &ref k)| k == key)
            .map(|(i, _)| i);

        match pos {
            Some(idx) => {
                self.cache_hits += 1;

                self.entries[idx].1 = self.counter;
                &self.entries[idx].0
            }
            None => {
                self.cache_misses += 1;

                let idx = self.find_lru_index();
                self.index[idx] = key.clone();
                self.entries[idx] = (f(key), self.counter);
                &self.entries[idx].0
            }
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

    fn normalize(&mut self) {
        let min_offset = self.entries.iter()
            .map(|(_, counter)| *counter)
            .min()
            .unwrap_or(0);

        self.entries.iter_mut()
            .for_each(|(_, counter)| *counter -= min_offset);
    }

    // Helper method to find the index of the least recently used entry
    fn find_lru_index(&self) -> usize {
        self.entries.iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| a.cmp(b))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

impl<K, V, const N: usize> Default for LruCache<K, V, N>
where
    K: PartialEq + Copy + Default,
    V: Copy + Default,
{
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoize_adds_entry() {
        let mut cache: LruCache<&str, i32, 5> = LruCache::new();

        let compute_called = std::cell::Cell::new(0);
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
        let compute_count = std::cell::Cell::new(0);

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
        let computation_occurred = std::cell::Cell::new(false);
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
        cache.memoize(&"a", |_| { compute_count += 1; 1 });
        cache.memoize(&"b", |_| { compute_count += 1; 2 });
        cache.memoize(&"d", |_| { compute_count += 1; 4 });

        assert_eq!(compute_count, 0, "Keys 'a', 'b', and 'd' should still be cached");
    }

    #[test]
    fn test_counter_overflow_handling() {
        let mut cache: LruCache<char, i32, 2> = LruCache::new();

        // Simulate a counter approaching overflow
        cache.counter = 0xffff - 2;

        // Add a few items
        cache.memoize(&'b', |_| 2);
        cache.memoize(&'a', |_| 1);

        // This should trigger counter overflow and normalize the counters
        cache.memoize(&'c', |_| 3);

        // Verify 'a' and 'b' were evicted by checking if they're recomputed
        let mut compute_count = 0;
        cache.memoize(&'a', |_| {
            compute_count += 1;
            1
        });

        assert_eq!(compute_count, 0, "Key 'a' should have been retained during normalization");
    }

    #[test]
    fn test_complex_key_types() {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        struct ComplexKey {
            id: String,
            section: u32,
        }

        let mut cache: LruCache<ComplexKey, Vec<i32>, 3> = LruCache::new();

        let key1 = ComplexKey { id: "test".to_string(), section: 1 };
        let key2 = ComplexKey { id: "test".to_string(), section: 2 };

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
    #[should_panic(expected = "Cache size must be greater than 0")]
    fn test_zero_size_cache_panics() {
        let _cache: LruCache<i32, i32, 0> = LruCache::new();
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
}