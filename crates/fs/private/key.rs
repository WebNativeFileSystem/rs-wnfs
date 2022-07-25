//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

use std::fmt::Debug;

use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce};
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct Key {
    bytes: Vec<u8>,
    cipher: Aes256Gcm,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl Key {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            cipher: Aes256Gcm::new(AesKey::from_slice(bytes.as_slice())),
        }
    }

    pub fn encrypt(&self, nonce: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        self.cipher
            .encrypt(Nonce::from_slice(nonce), data)
            .map_err(|e| anyhow!(e))
    }

    pub fn decrypt(&self, nonce: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        self.cipher
            .decrypt(Nonce::from_slice(nonce), data)
            .map_err(|e| anyhow!(e))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Key(0x{:02X?})", &self.bytes[..5])
    }
}
