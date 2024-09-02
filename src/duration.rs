#[cfg(feature = "std-duration")]
pub type Duration = std::time::Duration;

#[cfg(not(feature = "std-duration"))]
pub type Duration = duration::Duration;

#[cfg(not(feature = "std-duration"))]
pub mod duration {
    // Your custom Duration implementation goes here

    use std::iter::Sum;
    use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Duration {
        pub milliseconds: u32,
    }

    impl Duration {
        pub const ZERO: Self = Self { milliseconds: 0 };

        pub const fn from_millis(milliseconds: u32) -> Self {
            Self { milliseconds }
        }

        pub const fn from_secs(seconds: u32) -> Self {
            Self { milliseconds: seconds * 1000 }
        }

        pub fn from_secs_f32(seconds: f32) -> Self {
            Self { milliseconds: (seconds * 1000.0) as u32 }
        }

        pub fn as_millis(&self) -> u32 {
            self.milliseconds
        }

        pub const fn is_zero(&self) -> bool {
            self.milliseconds == 0
        }

        pub fn as_secs_f32(&self) -> f32 {
            self.milliseconds as f32 / 1000.0
        }

        pub fn checked_sub(&self, other: Self) -> Option<Self> {
            self.milliseconds
                .checked_sub(other.milliseconds)
                .map(Self::from_millis)
        }
    }

    impl Mul<u32> for Duration {
        type Output = Self;

        fn mul(self, rhs: u32) -> Self {
            Self { milliseconds: self.milliseconds * rhs }
        }
    }

    impl Add<Duration> for Duration {
        type Output = Self;

        fn add(self, rhs: Self) -> Self {
            Self { milliseconds: self.milliseconds + rhs.milliseconds }
        }
    }

    impl Add<u32> for Duration {
        type Output = Self;

        fn add(self, rhs: u32) -> Self {
            Self { milliseconds: self.milliseconds + rhs }
        }
    }

    impl AddAssign<Duration> for Duration {
        fn add_assign(&mut self, rhs: Self) {
            self.milliseconds += rhs.milliseconds;
        }
    }

    impl AddAssign<u32> for Duration {
        fn add_assign(&mut self, rhs: u32) {
            self.milliseconds += rhs;
        }
    }

    impl Sub<Duration> for Duration {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self {
            Self { milliseconds: self.milliseconds - rhs.milliseconds }
        }
    }

    impl Sub<u32> for Duration {
        type Output = Self;

        fn sub(self, rhs: u32) -> Self {
            Self { milliseconds: self.milliseconds - rhs }
        }
    }

    impl SubAssign<Duration> for Duration {
        fn sub_assign(&mut self, rhs: Self) {
            self.milliseconds -= rhs.milliseconds;
        }
    }

    impl SubAssign<u32> for Duration {
        fn sub_assign(&mut self, rhs: u32) {
            self.milliseconds -= rhs;
        }
    }

    impl Mul<Duration> for u32 {
        type Output = Duration;

        fn mul(self, rhs: Duration) -> Self::Output {
            Duration { milliseconds: self * rhs.milliseconds }
        }
    }

    impl Mul<f32> for Duration {
        type Output = Duration;

        fn mul(self, rhs: f32) -> Duration {
            Duration { milliseconds: (self.milliseconds as f32 * rhs) as u32 }
        }
    }

    impl Sum for Duration {
        fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
            iter.fold(Self::ZERO, Add::add)
        }
    }

    impl From<std::time::Duration> for Duration {
        fn from(d: std::time::Duration) -> Self {
            Self { milliseconds: d.as_millis() as u32 }
        }
    }

    impl From<Duration> for std::time::Duration {
        fn from(d: Duration) -> Self {
            std::time::Duration::from_millis(d.milliseconds as u64)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_duration_add() {
            let d1 = Duration::from_millis(100);
            let d2 = Duration::from_millis(200);
            assert_eq!(d1 + d2, Duration::from_millis(300));
        }

        #[test]
        fn verify_size_of_duration() {
            assert_eq!(size_of::<Duration>(), 4);
        }

        #[test]
        fn test_duration_sub() {
            let d1 = Duration::from_millis(200);
            let d2 = Duration::from_millis(100);
            assert_eq!(d1 - d2, Duration::from_millis(100));
        }

        #[test]
        fn test_duration_mul() {
            let d1 = Duration::from_millis(100);
            assert_eq!(d1 * 2, Duration::from_millis(200));
        }

        #[test]
        fn test_duration_sum() {
            let durations = vec![
                Duration::from_millis(100),
                Duration::from_millis(200),
                Duration::from_millis(300),
            ];
            assert_eq!(durations.iter().copied().sum::<Duration>(), Duration::from_millis(600));
        }

        #[test]
        fn test_duration_from_secs_f32() {
            let d = Duration::from_secs_f32(0.5);
            assert_eq!(d, Duration::from_millis(500));
        }

        #[test]
        fn test_duration_as_secs_f32() {
            let d = Duration::from_millis(500);
            assert_eq!(d.as_secs_f32(), 0.5);
        }

        #[test]
        fn test_duration_checked_sub() {
            let d1 = Duration::from_millis(200);
            let d2 = Duration::from_millis(100);
            assert_eq!(d1.checked_sub(d2), Some(Duration::from_millis(100)));
            assert_eq!(d2.checked_sub(d1), None);
        }
    }
}
