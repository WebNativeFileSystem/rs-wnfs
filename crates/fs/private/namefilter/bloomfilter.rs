use std::ops::Index;

use bitvec::prelude::BitArray;
use xxhash_rust::xxh3;

//------------------------------------------------------------------------------
// Type Definitions
//------------------------------------------------------------------------------

/// The bloom filter is a probabilistic data structure that can be used to store a set of hashes.
///
/// `N` is the size of the bloom filter in bytes.
///
/// `K` is the number of bits to be set with each add operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BloomFilter<const N: usize, const K: usize> {
    pub(super) bits: BitArray<[u8; N]>,
}

/// An iterator that generates indices into some bloom filter based on deterministic hashing of specified item.
///
/// `N` is the number of bytes in the bloom filter.
/// This is used to restrict generated value within bloomfilter index space bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashIndexIterator<'a, T: AsRef<[u8]>, const N: usize> {
    item: &'a T,
    index: u64,
}

//------------------------------------------------------------------------------
// Implementations
//------------------------------------------------------------------------------

impl<'a, T: AsRef<[u8]>, const N: usize> HashIndexIterator<'a, T, N> {
    /// Creates a new iterator.
    pub(super) fn new(item: &'a T) -> Self {
        Self { item, index: 0 }
    }

    /// Returns the size of the bloom filter in bits.
    pub const fn bit_size() -> usize {
        N * 8
    }
}

impl<T: AsRef<[u8]>, const N: usize> Iterator for HashIndexIterator<'_, T, N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let hash = xxh3::xxh3_64_with_seed(self.item.as_ref(), self.index) as usize;
        let value = hash % Self::bit_size();
        self.index += 1;
        Some(value)
    }
}

impl<const N: usize, const K: usize> BloomFilter<N, K> {
    /// Creates a new bloom filter with all bits unset.
    pub fn new() -> Self {
        Self {
            bits: Default::default(),
        }
    }

    /// Adds an item to the bloom filter.
    pub fn add<T>(&mut self, item: &T)
    where
        T: AsRef<[u8]>,
    {
        for i in self.hash_indices(item) {
            self.bits.set(i, true);
        }
    }

    /// Returns the number of hash iterations the bloom filter uses to set bits.
    pub const fn bit_index_count(&self) -> usize {
        K
    }

    /// Checks if the item is in the bloom filter.
    pub fn contains<T>(&self, item: &T) -> bool
    where
        T: AsRef<[u8]>,
    {
        for i in self.hash_indices(item) {
            if !self.bits[i] {
                return false;
            }
        }

        true
    }

    /// Counts the number of bits set in the bloom filter.
    pub fn count_ones(&self) -> usize {
        self.bits.count_ones()
    }

    /// Returns the indices of the bits that would be set if the item was added to the bloom filter.
    #[inline]
    pub fn hash_indices<'a, T>(&self, item: &'a T) -> impl Iterator<Item = usize> + 'a
    where
        T: AsRef<[u8]>,
    {
        HashIndexIterator::<_, N>::new(item).take(self.bit_index_count())
    }

    /// Get the bytes of the bloom filter.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.bits.as_raw_slice()
    }
}

impl<const N: usize, const K: usize> Index<usize> for BloomFilter<N, K> {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.bits[index]
    }
}

impl<const N: usize, const K: usize> Default for BloomFilter<N, K> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

//------------------------------------------------------------------------------
// Tests
//------------------------------------------------------------------------------

#[cfg(test)]
mod bloomfilter_tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn bloom_filter_can_add_and_validate_item_existence() {
        let mut bloom = BloomFilter::<256, 30>::new();
        let items: Vec<String> = vec!["first".into(), "second".into(), "third".into()];
        items.iter().for_each(|item| {
            bloom.add(item);
        });

        items.iter().for_each(|item| {
            assert!(bloom.contains(item));
        });

        assert!(!bloom.contains(b"irst"));
        assert!(!bloom.contains(b"secnd"));
        assert!(!bloom.contains(b"tird"));
    }

    #[test]
    fn iterator_can_give_unbounded_number_of_indices() {
        let iter = HashIndexIterator::<_, 200>::new(&"hello");

        let indices = (0..20)
            .map(|_| {
                let count = thread_rng().gen_range(0..500);
                (iter.clone().take(count).collect::<Vec<_>>(), count)
            })
            .collect::<Vec<_>>();

        for (indices, count) in indices {
            assert_eq!(indices.len(), count);
        }
    }
}
