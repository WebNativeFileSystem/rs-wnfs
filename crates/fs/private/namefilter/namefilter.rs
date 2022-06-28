use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake256,
};

use super::BloomFilter;

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

pub const SATURATION_THRESHOLD: usize = 1019;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// Namefilters are 2048-bit bloom filters.
///
/// In WNFS they represent the identity of a file or directory, doubling as a store for checking the ancestor of the file or directory.
pub type Namefilter = BloomFilter<256, 30>;

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl Namefilter {
    /// Creates a saturated namefilter.
    pub fn new_saturated() -> Self {
        let mut filter = Self::new();
        filter.saturate();
        filter
    }

    /// Adds hashes to filter until is is over the saturation threshold.
    fn saturate(&mut self) {
        let mut xof = {
            let mut h = Shake256::default();
            h.update(&self.as_bytes());
            h.finalize_xof()
        };

        while self.count_ones() <= SATURATION_THRESHOLD {
            let buffer = &mut [0u8; 32];
            xof.read(buffer);
            self.add(buffer);
        }
    }
}

impl AsRef<[u8]> for Namefilter {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod namefilter_tests {
    use super::*;

    #[test]
    fn saturate_not_less_than_threshold() {
        let namefilters = (0..100)
            .map(|_| Namefilter::new_saturated())
            .collect::<Vec<_>>();

        for namefilter in namefilters {
            assert!(namefilter.count_ones() >= SATURATION_THRESHOLD);
        }
    }
}
