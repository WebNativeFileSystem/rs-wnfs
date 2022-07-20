use anyhow::Result;
use async_trait::async_trait;

use crate::{BlockStore, ReferenceableStore, AsyncSerialize};

use super::{EncryptedPrivateNode, Hamt, Namefilter, PrivateNode, PrivateRef};

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

    /// Sets a new value at the given key.
    #[inline]
    pub async fn set(&mut self, key: &PrivateRef, value: PrivateNode) -> Result<()> {
        // TODO(appcypher): Fix this.
        // let value_bytes = value.async_serialize(self.store).await?;
        // let enc_value = key.content_key.encrypt(value_bytes).await?;
        todo!()
    }

    /// Gets the value at the given key.
    #[inline]
    pub async fn get(&self, key: &PrivateRef) -> Result<Option<PrivateNode>> {
        todo!()
    }

    /// Removes the value at the given key.
    pub async fn remove(&mut self, key: &PrivateRef) -> Result<Option<PrivateNode>> {
        todo!()
    }

    /// Sets a new encrypted value at the given key.
    #[inline]
    pub async fn set_encrypted(
        &mut self,
        key: Namefilter,
        value: EncryptedPrivateNode,
    ) -> Result<()> {
        let root = self.root.root.set(key, value, self.store).await?;
        self.root.root = root;
        Ok(())
    }

    /// Gets the encrypted value at the given key.
    #[inline]
    pub async fn get_encrypted<'b>(
        &'b self,
        key: &Namefilter,
    ) -> Result<Option<&'b EncryptedPrivateNode>> {
        self.root.root.get(key, self.store).await
    }

    /// Removes the encrypted value at the given key.
    pub async fn remove_encrypted(
        &mut self,
        key: &Namefilter,
    ) -> Result<Option<EncryptedPrivateNode>> {
        let (root, value) = self.root.root.remove(key, self.store).await?;
        self.root.root = root;
        Ok(value)
    }
}

#[async_trait(?Send)]
impl<B: BlockStore> ReferenceableStore<PrivateNode> for HamtStore<'_, B> {
    type Reference = Namefilter;

    async fn get_value(&self, reference: &Self::Reference) -> Result<PrivateNode> {
        todo!()
    }

    async fn put_value(&mut self, value: &PrivateNode) -> Result<Self::Reference> {
        todo!()
    }
}
