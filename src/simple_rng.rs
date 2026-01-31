use core::ops::Range;
#[cfg(all(feature = "std", not(feature = "wasm")))]
use std::time::SystemTime;

#[cfg(feature = "wasm")]
use web_time::SystemTime;

/// A simple pseudo-random number generator using the SplitMix32 algorithm.
///
/// SplitMix32 is a fast, high-quality PRNG with good statistical properties.
/// It passes most statistical tests including matrix rank and has low
/// serial correlation. Not suitable for cryptographic purposes.
///
/// # Examples
///
/// ```
/// use tachyonfx::SimpleRng;
///
/// let mut rng = SimpleRng::new(12345);
/// let random_u32 = rng.gen();
/// let random_float = rng.gen_f32();
/// println!("u32={} f32={}", random_u32, random_float);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    pub fn new(seed: u32) -> Self {
        SimpleRng { state: seed }
    }

    /// Generates the next pseudo-random u32 value using SplitMix32.
    ///
    /// This method updates the internal state and returns a well-mixed value.
    ///
    /// # Returns
    ///
    /// A pseudo-random u32 value.
    #[inline]
    pub fn gen(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9E3779B9);
        let mut z = self.state;
        z = (z ^ (z >> 15)).wrapping_mul(0x85EBCA6B);
        z = (z ^ (z >> 13)).wrapping_mul(0xC2B2AE35);
        z ^ (z >> 16)
    }

    /// Generates a pseudo-random f32 value in the range [0, 1).
    ///
    /// This method uses bit manipulation for efficiency, generating
    /// uniformly distributed float values.
    ///
    /// # Returns
    ///
    /// A pseudo-random f32 value in the range [0, 1).
    pub fn gen_f32(&mut self) -> f32 {
        const EXPONENT: u32 = 0x3f800000; // 1.0f32
        let mantissa = self.gen() >> 9; // 23 bits of randomness

        f32::from_bits(EXPONENT | mantissa) - 1.0
    }

    #[cfg(target_pointer_width = "64")]
    fn gen_usize(&mut self) -> usize {
        let mut g = || self.gen() as usize;
        (g() << 32) | g()
    }

    #[cfg(target_pointer_width = "32")]
    fn gen_usize(&mut self) -> usize {
        self.gen() as usize
    }
}

#[cfg(any(feature = "std", feature = "wasm"))]
impl Default for SimpleRng {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32;

        SimpleRng::new(seed)
    }
}

#[cfg(not(any(feature = "std", feature = "wasm")))]
impl Default for SimpleRng {
    fn default() -> Self {
        // Use a fixed seed in no-std environments where SystemTime is unavailable
        SimpleRng::new(0x12345678)
    }
}

pub trait RangeSampler<T> {
    fn gen_range(&mut self, range: Range<T>) -> T;
}

impl RangeSampler<u16> for SimpleRng {
    fn gen_range(&mut self, range: Range<u16>) -> u16 {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + (self.gen() >> 16) as u16 % range_size
    }
}

impl RangeSampler<u32> for SimpleRng {
    fn gen_range(&mut self, range: Range<u32>) -> u32 {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + self.gen() % range_size
    }
}

impl RangeSampler<usize> for SimpleRng {
    fn gen_range(&mut self, range: Range<usize>) -> usize {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + self.gen_usize() % range_size
    }
}

impl RangeSampler<f32> for SimpleRng {
    fn gen_range(&mut self, range: Range<f32>) -> f32 {
        let range_size = range.end - range.start;
        assert!(
            range_size > 0.0,
            "range.end must be greater than range.start"
        );

        range.start + self.gen_f32() % range_size
    }
}

impl RangeSampler<i16> for SimpleRng {
    fn gen_range(&mut self, range: Range<i16>) -> i16 {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + (self.gen() % range_size as u32) as i16
    }
}

impl RangeSampler<i32> for SimpleRng {
    fn gen_range(&mut self, range: Range<i32>) -> i32 {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + (self.gen() % range_size as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    use std::panic;

    use super::*;

    #[cfg(feature = "std")]
    const RETRY_COUNT: usize = 5;

    #[cfg(feature = "std")]
    fn run_test<F>(test: F)
    where
        F: Fn() + panic::RefUnwindSafe,
    {
        let mut success = false;
        for _ in 0..RETRY_COUNT {
            if panic::catch_unwind(&test).is_ok() {
                success = true;
                break;
            }
        }
        assert!(success, "Test failed after {RETRY_COUNT} attempts");
    }

    #[cfg(not(feature = "std"))]
    fn run_test<F>(test: F)
    where
        F: Fn(),
    {
        // In no-std environments, we can't catch panics, so just run the test once
        test();
    }

    #[test]
    fn test_reproducibility() {
        let mut rng1 = SimpleRng::new(12345);
        let mut rng2 = SimpleRng::new(12345);

        for _ in 0..100 {
            assert_eq!(rng1.gen(), rng2.gen());
        }
    }

    #[test]
    fn test_different_seeds() {
        let mut rng1 = SimpleRng::new(12345);
        let mut rng2 = SimpleRng::new(54321);

        assert_ne!(rng1.gen(), rng2.gen());
    }

    #[test]
    fn test_gen_f32_range() {
        let mut rng = SimpleRng::new(12345);

        for _ in 0..1000 {
            let value = rng.gen_f32();
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn test_gen_range_u32() {
        let mut rng = SimpleRng::new(12345);
        let range = 10..20;

        for _ in 0..1000 {
            let value = rng.gen_range(range.clone());
            assert!((10..20).contains(&value));
        }
    }

    #[test]
    fn test_gen_range_f32() {
        let mut rng = SimpleRng::new(12345);
        let range = 0.0..1.0;

        for _ in 0..1000 {
            let value = rng.gen_range(range.clone());
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    #[should_panic(expected = "range.end must be greater than range.start")]
    fn test_gen_range_invalid() {
        let mut rng = SimpleRng::new(12345);
        #[allow(clippy::reversed_empty_ranges)]
        rng.gen_range(20..10);
    }

    #[test]
    fn test_overflow_handling() {
        let mut rng = SimpleRng::new(u32::MAX);

        // This should not panic
        rng.gen();
    }

    #[test]
    #[allow(clippy::unnecessary_cast)] // misidentified by clippy
    fn test_uniform_distribution_u32() {
        run_test(|| {
            let mut rng = SimpleRng::new(12345);
            let mut counts = [0; 10];
            let num_samples = 100000;

            for _ in 0..num_samples {
                let value = rng.gen_range(0..10);
                counts[value as usize] += 1;
            }

            let expected = num_samples / 10;
            for &count in &counts {
                assert!(
                    (count as i32 - expected as i32).abs() < 500,
                    "Distribution is not uniform: {counts:?}"
                );
            }
        });
    }

    #[test]
    #[allow(clippy::unnecessary_cast)] // misidentified by clippy
    fn test_uniform_distribution_f32() {
        run_test(|| {
            let mut rng = SimpleRng::new(12345);
            let mut counts = [0; 10];
            let num_samples = 100000;

            for _ in 0..num_samples {
                let value = rng.gen_range(0.0..1.0);
                let bucket = (value * 10.0) as usize;
                counts[bucket.min(9)] += 1;
            }

            let expected = num_samples / 10;
            for &count in &counts {
                assert!(
                    (count as i32 - expected as i32).abs() < 500,
                    "Distribution is not uniform: {counts:?}"
                );
            }
        });
    }

    #[test]
    #[cfg(any(feature = "std", feature = "wasm"))] // Only run when we have SystemTime
    #[allow(clippy::std_instead_of_core)]
    fn test_default_rng() {
        let rng1 = SimpleRng::default();
        #[cfg(feature = "std")]
        {
            let duration = std::time::Duration::from_millis(10);
            std::thread::sleep(duration);
        }
        #[cfg(all(feature = "wasm", not(feature = "std")))]
        {
            // In web environments, we can't sleep, but we can just create another RNG
            // The timestamp should be different enough to produce different seeds
        }
        let rng2 = SimpleRng::default();

        assert_ne!(
            rng1.state, rng2.state,
            "Default RNGs should have different seeds"
        );
    }

    #[test]
    fn test_gen_usize() {
        let mut rng = SimpleRng::new(12345);
        let value = rng.gen_usize();
        assert!(value > 0, "gen_usize should generate non-zero values");
    }

    #[test]
    fn test_gen_range_i32() {
        let mut rng = SimpleRng::new(12345);
        let range = -10..10;

        for _ in 0..1000 {
            let value = rng.gen_range(range.clone());
            assert!(range.contains(&value));
        }
    }
}
