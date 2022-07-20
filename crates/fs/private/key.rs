use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[async_trait]
pub trait Key: Debug {
    async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
    async fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
}
