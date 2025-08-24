use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr, Not};

/// A simple bit vector implementation using a fixed size determined at construction.
#[derive(Debug, Clone)]
pub(crate) struct BitVec {
    data: Vec<u32>,
}

impl BitVec {
    /// Creates a new empty BitVec.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Resizes the BitVec to the specified length, filling new elements with the given
    /// value.
    pub fn resize(&mut self, new_len: usize, value: bool) {
        let word_len = new_len.div_ceil(32);
        self.data
            .resize(word_len, if value { 0xFFFFFFFF } else { 0x00000000 });
    }

    /// Returns the number of bits in the BitVec.
    pub fn len(&self) -> usize {
        self.data.len() * 32
    }

    /// Sets the bit at the given index to the specified value.
    pub fn set(&mut self, index: usize, value: bool) {
        if index >= self.len() {
            return;
        }

        let word_index = index / 32;
        let bit_index = index % 32;

        if value {
            self.data[word_index] |= 1 << bit_index;
        } else {
            self.data[word_index] &= !(1 << bit_index);
        }
    }

    /// Gets the bit at the given index.
    pub fn get(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }

        let word_index = index / 32;
        let bit_index = index % 32;

        (self.data[word_index] & (1 << bit_index)) != 0
    }

    /// Checks if the BitVec is empty (all bits are false).
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&word| word == 0)
    }
}

impl core::ops::Index<usize> for BitVec {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        // We can't actually return a &bool here because the bool is computed
        // Instead, we'll use a workaround with static references
        if self.get(index) {
            &true
        } else {
            &false
        }
    }
}

impl BitAnd for BitVec {
    type Output = BitVec;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        if rhs.len() < self.len() {
            self.resize(rhs.len(), false);
        }

        self.data
            .iter_mut()
            .zip(rhs.data.iter())
            .for_each(|(a, b)| *a &= *b);
        self
    }
}

impl BitOr for BitVec {
    type Output = BitVec;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        if self.len() < rhs.len() {
            self.resize(rhs.len(), false);
        }

        self.data
            .iter_mut()
            .zip(rhs.data.iter())
            .for_each(|(a, b)| *a |= *b);
        self
    }
}

impl Not for BitVec {
    type Output = BitVec;

    fn not(mut self) -> Self::Output {
        for word in &mut self.data {
            *word = !*word;
        }

        self
    }
}

impl Default for BitVec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bitvec() {
        let bv = BitVec::new();
        assert_eq!(bv.len(), 0);
        assert!(bv.is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let mut bv = BitVec::new();
        bv.resize(8, false);

        bv.set(0, true);
        bv.set(3, true);
        bv.set(7, true);

        assert!(bv.get(0));
        assert!(!bv.get(1));
        assert!(!bv.get(2));
        assert!(bv.get(3));
        assert!(!bv.get(4));
        assert!(!bv.get(5));
        assert!(!bv.get(6));
        assert!(bv.get(7));
    }

    #[test]
    fn test_index() {
        let mut bv = BitVec::new();
        bv.resize(5, false);
        bv.set(2, true);

        assert!(!bv[0]);
        assert!(!bv[1]);
        assert!(bv[2]);
        assert!(!bv[3]);
        assert!(!bv[4]);
    }

    #[test]
    fn test_bitand() {
        let mut bv1 = BitVec::new();
        bv1.resize(4, false);
        bv1.set(0, true);
        bv1.set(1, true);
        bv1.set(2, false);
        bv1.set(3, true);

        let mut bv2 = BitVec::new();
        bv2.resize(4, false);
        bv2.set(0, true);
        bv2.set(1, false);
        bv2.set(2, true);
        bv2.set(3, true);

        let result = bv1.bitand(bv2);
        assert!(result[0]); // true & true = true
        assert!(!result[1]); // true & false = false
        assert!(!result[2]); // false & true = false
        assert!(result[3]); // true & true = true
    }

    #[test]
    fn test_bitor() {
        let mut bv1 = BitVec::new();
        bv1.resize(4, false);
        bv1.set(0, true);
        bv1.set(1, false);
        bv1.set(2, false);
        bv1.set(3, true);

        let mut bv2 = BitVec::new();
        bv2.resize(4, false);
        bv2.set(0, false);
        bv2.set(1, true);
        bv2.set(2, false);
        bv2.set(3, true);

        let result = bv1.bitor(bv2);
        assert!(result[0]); // true | false = true
        assert!(result[1]); // false | true = true
        assert!(!result[2]); // false | false = false
        assert!(result[3]); // true | true = true
    }

    #[test]
    fn test_not() {
        let mut bv = BitVec::new();
        bv.resize(4, false);
        bv.set(0, true);
        bv.set(2, true);

        let result = bv.not();
        assert!(!result[0]); // !true = false
        assert!(result[1]); // !false = true
        assert!(!result[2]); // !true = false
        assert!(result[3]); // !false = true
    }
}
