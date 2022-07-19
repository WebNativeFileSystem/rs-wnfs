use anyhow::Result;

use crate::BlockStore;

use super::{EncryptedPrivateNode, Hamt, Namefilter};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type PrivateRoot = Hamt<Namefilter, EncryptedPrivateNode>;

pub struct HamtStore<'a, B: BlockStore> {
    pub root: PrivateRoot,
    pub store: &'a mut B,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<'a, B: BlockStore> HamtStore<'a, B> {
    /// Creates a new HamtStore.
    pub fn new(store: &'a mut B) -> Self {
        Self {
            root: Hamt::default(),
            store,
        }
    }

    /// TODO(appcypher): PrivateRef and PrivateNodeSchema<T>
    /// Sets a new value at the given key.
    #[inline]
    pub async fn set(&mut self, key: Namefilter, value: EncryptedPrivateNode) -> Result<()> {
        let root = self.root.root.set(key, value, self.store).await?;
        self.root.root = root;
        Ok(())
    }

    /// TODO(appcypher): PrivateRef and PrivateNodeSchema<T>
    /// Gets the value at the given key.
    #[inline]
    pub async fn get<'b>(&'b self, key: &Namefilter) -> Result<Option<&'b EncryptedPrivateNode>> {
        self.root.root.get(key, self.store).await
    }

    /// TODO(appcypher): PrivateRef and PrivateNodeSchema<T>
    /// Removes the value at the given key.
    pub async fn remove(&mut self, key: &Namefilter) -> Result<Option<EncryptedPrivateNode>> {
        let (root, value) = self.root.root.remove(key, self.store).await?;
        self.root.root = root;
        Ok(value)
    }
}
