use anyhow::{bail, Result};
use sha3::{Digest, Sha3_256};

use crate::{HashOutput, HASH_BYTE_SIZE};

use super::error::HamtError;

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

const MAX_CURSOR_DEPTH: usize = HASH_BYTE_SIZE * 2;

//--------------------------------------------------------------------------------------------------
// Type Definition
//--------------------------------------------------------------------------------------------------

/// A common trait for the ability to generate a hash of some data.
pub trait Hasher {
    /// Generates a hash of the given data.
    fn hash<K: AsRef<[u8]>>(key: &K) -> HashOutput;
}

#[derive(Debug, Clone)]
pub struct HashNibbles<'a> {
    digest: &'a HashOutput,
    cursor: usize,
}

//--------------------------------------------------------------------------------------------------
// Implementation
//--------------------------------------------------------------------------------------------------

impl<'a> HashNibbles<'a> {
    /// Creates a new `HashNibbles` instance from a `[u8; 32]` hash.
    pub fn new(digest: &'a HashOutput) -> HashNibbles<'a> {
        Self::with_cursor(digest, 0)
    }

    /// Constructs hash nibbles with custom cursor index.
    pub fn with_cursor(digest: &'a HashOutput, cursor: usize) -> HashNibbles<'a> {
        Self { digest, cursor }
    }

    /// Gets the next nibble from the hash.
    pub fn try_next(&mut self) -> Result<usize> {
        if let Some(nibble) = self.next() {
            return Ok(nibble as usize);
        }
        bail!(HamtError::CursorOutOfBounds)
    }

    /// Gets the current cursor position.
    #[inline]
    pub fn get_cursor(&self) -> usize {
        self.cursor
    }
}

impl Iterator for HashNibbles<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= MAX_CURSOR_DEPTH {
            return None;
        }

        let byte = self.digest[self.cursor / 2];
        let byte = if self.cursor % 2 == 0 {
            byte >> 4
        } else {
            byte & 0b0000_1111
        };

        self.cursor += 1;
        Some(byte)
    }
}

impl Hasher for Sha3_256 {
    fn hash<K: AsRef<[u8]>>(key: &K) -> HashOutput {
        let mut hasher = Self::default();
        hasher.update(key.as_ref());
        hasher.finalize().into()
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod hash_nibbles_tests {
    use super::*;

    #[test]
    fn hash_nibbles_can_cursor_over_digest() {
        let key = {
            let mut bytes = [0u8; HASH_BYTE_SIZE];
            bytes[0] = 0b1000_1100;
            bytes[1] = 0b1010_1010;
            bytes[2] = 0b1011_1111;
            bytes[3] = 0b1111_1101;
            bytes
        };

        let hashnibbles = &mut HashNibbles::new(&key);
        let expected_nibbles = [
            0b1000, 0b1100, 0b1010, 0b1010, 0b1011, 0b1111, 0b1111, 0b1101,
        ];

        for (got, expected) in hashnibbles.zip(expected_nibbles.into_iter()) {
            assert_eq!(expected, got);
        }

        // Exhaust the iterator.
        let _ = hashnibbles
            .take(MAX_CURSOR_DEPTH - expected_nibbles.len())
            .collect::<Vec<_>>();

        assert_eq!(hashnibbles.next(), None);
    }
}
