use crate::BlockStore;

use anyhow::Result;
use async_trait::async_trait;

//--------------------------------------------------------------------------------------------------
// Traits
//--------------------------------------------------------------------------------------------------

/// Implements getting a unique identifier for a node.
pub trait Id {
    /// Gets an identifier for the node.
    fn get_id(&self) -> String;
}

/// Implements deep equality check for two types.
#[async_trait(?Send)]
pub trait IpldEq {
    /// Checks if the two items are deeply equal.
    async fn eq<B: BlockStore>(&self, other: &Self, store: &mut B) -> Result<bool>;
}
