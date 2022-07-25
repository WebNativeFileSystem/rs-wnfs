use anyhow::Result;
use async_trait::async_trait;
use libipld::Cid;
use serde::de::DeserializeOwned;

use crate::{AsyncSerialize, BlockStore, ReferenceableStore};

use super::{Hamt, Namefilter, PrivateNode, PrivateRef};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type EncryptedPrivateNode = (Vec<u8>, Cid); // (enc_header_bytes, main_cid -> enc_main_bytes)
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
        todo!()
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
impl<B: BlockStore> ReferenceableStore for HamtStore<'_, B> {
    type Ref = PrivateRef;

    async fn get_value<V: DeserializeOwned>(&self, reference: &Self::Ref) -> Result<V> {
        todo!()
    }

    async fn put_value<V: AsyncSerialize>(&mut self, value: &V) -> Result<Self::Ref> {
        todo!()
    }
}
