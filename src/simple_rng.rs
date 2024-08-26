use std::ops::Range;
use std::time::SystemTime;

/// A simple pseudo-random number generator using the Linear Congruential Generator algorithm.
///
/// This RNG is fast and uses minimal memory, and is definitely not suitable for
/// cryptographic purposes or high-quality randomness.
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
#[derive(Clone, Copy)]
pub struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    const A: u32 = 1664525;
    const C: u32 = 1013904223;

    pub fn new(seed: u32) -> Self {
        SimpleRng { state: seed }
    }

    /// Generates the next pseudo-random u32 value.
    ///
    /// This method updates the internal state and returns the new value.
    ///
    /// # Returns
    ///
    /// A pseudo-random u32 value.
    pub fn gen(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(Self::A).wrapping_add(Self::C);
        self.state
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
        let mantissa = self.gen() >> 9;   // 23 bits of randomness

        f32::from_bits(EXPONENT | mantissa) - 1.0
    }

    fn gen_usize(&mut self) -> usize {
        let mut g = || self.gen() as usize;
        g() << 32 | g()
    }
}

impl Default for SimpleRng {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32;

        SimpleRng::new(seed)
    }
}

pub trait RangeSampler<T> {
    fn gen_range(&mut self, range: Range<T>) -> T;
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
        assert!(range_size > 0.0, "range.end must be greater than range.start");

        range.start + self.gen_f32() % range_size
    }
}

impl RangeSampler<i32> for SimpleRng {
    fn gen_range(&mut self, range: Range<i32>) -> i32 {
        let range_size = range.end.wrapping_sub(range.start);
        assert!(range_size > 0, "range.end must be greater than range.start");

        range.start + (self.gen() % range_size as u32) as i32
    }
}

pub fn shuffle<T>(vec: &mut Vec<T>, rng: &mut SimpleRng) {
    let len = vec.len();
    for i in 0..len {
        let j = rng.gen_range(i..len);
        vec.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use std::panic;
    use super::*;

    const RETRY_COUNT: usize = 5;

    fn run_test<F>(test: F)
    where
        F: Fn() + panic::RefUnwindSafe,
    {
        let mut success = false;
        for _ in 0..RETRY_COUNT {
            if panic::catch_unwind(|| test()).is_ok() {
                success = true;
                break;
            }
        }
        assert!(success, "Test failed after {} attempts", RETRY_COUNT);
    }

    #[test]
    fn test_lcg_reproducibility() {
        let mut lcg1 = SimpleRng::new(12345);
        let mut lcg2 = SimpleRng::new(12345);

        for _ in 0..100 {
            assert_eq!(lcg1.gen(), lcg2.gen());
        }
    }

    #[test]
    fn test_lcg_different_seeds() {
        let mut lcg1 = SimpleRng::new(12345);
        let mut lcg2 = SimpleRng::new(54321);

        assert_ne!(lcg1.gen(), lcg2.gen());
    }

    #[test]
    fn test_gen_f32_range() {
        let mut lcg = SimpleRng::new(12345);

        for _ in 0..1000 {
            let value = lcg.gen_f32();
            assert!(value >= 0.0 && value < 1.0);
        }
    }

    #[test]
    fn test_gen_range_u32() {
        let mut lcg = SimpleRng::new(12345);
        let range = 10..20;

        for _ in 0..1000 {
            let value = lcg.gen_range(range.clone());
            assert!(value >= 10 && value < 20);
        }
    }

    #[test]
    fn test_gen_range_f32() {
        let mut lcg = SimpleRng::new(12345);
        let range = 0.0..1.0;

        for _ in 0..1000 {
            let value = lcg.gen_range(range.clone());
            assert!(value >= 0.0 && value < 1.0);
        }
    }

    #[test]
    #[should_panic(expected = "range.end must be greater than range.start")]
    fn test_gen_range_invalid() {
        let mut lcg = SimpleRng::new(12345);
        lcg.gen_range(20..10);
    }

    #[test]
    fn test_shuffle() {
        let mut lcg = SimpleRng::new(12345);
        let mut vec = vec![1, 2, 3, 4, 5];
        let original = vec.clone();

        shuffle(&mut vec, &mut lcg);

        assert_ne!(vec, original);
        assert_eq!(vec.len(), original.len());
        assert_eq!(vec.iter().sum::<i32>(), original.iter().sum::<i32>());
    }

    #[test]
    fn test_lcg_overflow_handling() {
        let mut lcg = SimpleRng::new(u32::MAX);

        // This should not panic
        lcg.gen();
    }

    #[test]
    fn test_uniform_distribution_u32() {
        run_test(|| {
            let mut lcg = SimpleRng::new(12345);
            let mut counts = [0; 10];
            let num_samples = 100000;

            for _ in 0..num_samples {
                let value = lcg.gen_range(0..10);
                counts[value as usize] += 1;
            }

            let expected = num_samples / 10;
            for &count in &counts {
                assert!((count as i32 - expected as i32).abs() < 500,
                    "Distribution is not uniform: {:?}", counts);
            }
        });
    }

    #[test]
    fn test_uniform_distribution_f32() {
        run_test(|| {
            let mut lcg = SimpleRng::new(12345);
            let mut counts = [0; 10];
            let num_samples = 100000;

            for _ in 0..num_samples {
                let value = lcg.gen_range(0.0..1.0);
                let bucket = (value * 10.0) as usize;
                counts[bucket.min(9)] += 1;
            }

            let expected = num_samples / 10;
            for &count in &counts {
                assert!((count as i32 - expected as i32).abs() < 500,
                    "Distribution is not uniform: {:?}", counts);
            }
        });
    }

    #[test]
    fn test_default_lcg() {
        let lcg1 = SimpleRng::default();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let lcg2 = SimpleRng::default();

        assert_ne!(lcg1.state, lcg2.state, "Default LCGs should have different seeds");
    }

    #[test]
    fn test_gen_usize() {
        let mut lcg = SimpleRng::new(12345);
        let value = lcg.gen_usize();
        assert!(value > 0, "gen_usize should generate non-zero values");
    }

    #[test]
    fn test_gen_range_i32() {
        let mut lcg = SimpleRng::new(12345);
        let range = -10..10;

        for _ in 0..1000 {
            let value = lcg.gen_range(range.clone());
            assert!(value >= -10 && value < 10);
        }
    }
}