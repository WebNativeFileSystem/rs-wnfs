use anyhow::Result;
use async_trait::async_trait;
use libipld::Cid;
use serde::de::DeserializeOwned;

use crate::{AsyncSerialize, Referenceable, ReferenceableStore};
use crate::{BlockStore, IpldEq};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// A data structure that represents a link in the IPLD graph. Basically it is "link" to some content addressable value of `T`.
///
/// It supports representing the "link" with a Cid or the deserialized value itself.
///
/// Link needs a `BlockStore` to be able to resolve Cids to corresponding values of `T` and vice versa.
pub type Link<T> = Referenceable<Cid, T>;

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<T> Link<T> {
    /// Creates a new `Link` that starts out as a Cid.
    #[inline]
    pub fn from_cid(cid: Cid) -> Self {
        Self::from_reference(cid)
    }

    /// Gets the Cid stored in link. It attempts to get it from the store if it is not present in link.
    #[inline]
    pub async fn resolve_cid<'a, B: BlockStore + ?Sized>(&'a self, store: &mut B) -> Result<&'a Cid>
    where
        T: AsyncSerialize,
    {
        self.resolve_reference(store).await
    }

    /// Checks if there is a Cid stored in link.
    #[inline]
    pub fn has_cid(&self) -> bool {
        self.has_reference()
    }

    /// Compares two links for equality. Attempts to get them from store if they are not already cached.
    pub async fn deep_eq<B: BlockStore>(&self, other: &Link<T>, store: &mut B) -> Result<bool>
    where
        T: PartialEq + AsyncSerialize,
    {
        if self == other {
            return Ok(true);
        }

        Ok(self.resolve_cid(store).await? == other.resolve_cid(store).await?)
    }
}

#[async_trait(?Send)]
impl<T: PartialEq + AsyncSerialize> IpldEq for Link<T> {
    async fn eq<B: BlockStore>(&self, other: &Link<T>, store: &mut B) -> Result<bool> {
        if self == other {
            return Ok(true);
        }

        Ok(self.resolve_cid(store).await? == other.resolve_cid(store).await?)
    }
}

#[async_trait(?Send)]
impl<V, T: BlockStore + ?Sized> ReferenceableStore<V> for T {
    type Reference = Cid;

    async fn get_value(&self, reference: &Self::Reference) -> Result<V> where V: DeserializeOwned {
        self.get_deserializable(reference).await
    }

    async fn put_value(&mut self, value: &V) -> Result<Self::Reference> where V: AsyncSerialize {
        self.put_async_serializable(value).await
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod ipld_link_tests {
    use crate::{BlockStore, Link, MemoryBlockStore};

    #[async_std::test]
    async fn link_value_can_be_resolved() {
        let store = &mut MemoryBlockStore::default();
        let cid = store.put_serializable(&256).await.unwrap();
        let link = Link::<u64>::from_cid(cid);

        let value = link.resolve_value(store).await.unwrap();
        assert_eq!(value, &256);
        assert!(link.has_value());
    }

    #[async_std::test]
    async fn link_cid_can_be_resolved() {
        let pair = ("price".into(), 12_000_500);
        let store = &mut MemoryBlockStore::default();
        let link = Link::<(String, u64)>::from(pair.clone());

        let cid = link.resolve_cid(store).await.unwrap();
        let value = store
            .get_deserializable::<(String, u64)>(cid)
            .await
            .unwrap();

        assert_eq!(value, pair);
    }
}
