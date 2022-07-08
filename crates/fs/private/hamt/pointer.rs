// TODO(appcypher): Based on ipld_hamt implementation

use std::rc::Rc;

use anyhow::Result;
use async_trait::async_trait;
use libipld::{serde as ipld_serde, Ipld};
use serde::{
    de::{DeserializeOwned, Error as DeError},
    ser::Error as SerError,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{AsyncSerialize, BlockStore, Link};

use super::{hash::Hasher, Node};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Debug, Clone)]
pub enum Pointer<K, V, H>
where
    H: Hasher,
{
    Values(Vec<Pair<K, V>>),
    Link(Link<Rc<Node<K, V, H>>>),
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<K, V> Pair<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

impl<K, V, H: Hasher> Pointer<K, V, H> {
    /// Converts a Pointer to an IPLD object.
    pub async fn to_ipld<B: BlockStore + ?Sized>(&self, store: &mut B) -> Result<Ipld>
    where
        K: Serialize,
        V: Serialize,
    {
        Ok(match self {
            Pointer::Values(values) => ipld_serde::to_ipld(values)?,
            Pointer::Link(link) => ipld_serde::to_ipld(link.resolve_cid(store).await?)?,
        })
    }
}

#[async_trait(?Send)]
impl<K, V, H: Hasher> AsyncSerialize for Pointer<K, V, H>
where
    K: Serialize,
    V: Serialize,
{
    async fn async_serialize<S: Serializer, B: BlockStore + ?Sized>(
        &self,
        serializer: S,
        store: &mut B,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Pointer::Values(vals) => vals.serialize(serializer),
            Pointer::Link(link) => link
                .resolve_cid(store)
                .await
                .map_err(SerError::custom)?
                .serialize(serializer),
        }
    }
}

impl<'de, K, V, H: Hasher> Deserialize<'de> for Pointer<K, V, H>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ipld::deserialize(deserializer).and_then(|ipld| ipld.try_into().map_err(DeError::custom))
    }
}

impl<K, V, H: Hasher> TryFrom<Ipld> for Pointer<K, V, H>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    type Error = String;

    fn try_from(ipld: Ipld) -> Result<Self, Self::Error> {
        match ipld {
            ipld_list @ Ipld::List(_) => {
                let values: Vec<Pair<K, V>> =
                    Deserialize::deserialize(ipld_list).map_err(|error| error.to_string())?;
                Ok(Self::Values(values))
            }
            Ipld::Link(cid) => Ok(Self::Link(Link::from_cid(cid))),
            other => Err(format!(
                "Expected `Ipld::List` or `Ipld::Link`, got {:#?}",
                other
            )),
        }
    }
}

impl<K, V, H: Hasher> Default for Pointer<K, V, H> {
    fn default() -> Self {
        Pointer::Values(Vec::new())
    }
}

impl<K, V, H: Hasher> PartialEq for Pointer<K, V, H>
where
    K: PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Pointer::Values(vals), Pointer::Values(other_vals)) => vals == other_vals,
            (Pointer::Link(link), Pointer::Link(other_link)) => link == other_link,
            _ => false,
        }
    }
}

impl<'de, K, V> Deserialize<'de> for Pair<K, V>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (key, value) = <(K, V)>::deserialize(deserializer)?;
        Ok(Pair { key, value })
    }
}

impl<K, V> Serialize for Pair<K, V>
where
    K: Serialize,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.key, &self.value).serialize(serializer)
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod pointer_tests {
    use sha3::Sha3_256;

    use super::*;
    use crate::{dagcbor, MemoryBlockStore};

    #[async_std::test]
    async fn pointer_can_encode_decode_as_cbor() {
        let store = &mut MemoryBlockStore::default();
        let pointer: Pointer<String, i32, Sha3_256> = Pointer::Values(vec![
            Pair {
                key: "James".into(),
                value: 4500,
            },
            Pair {
                key: "Peter".into(),
                value: 2000,
            },
        ]);

        let encoded_pointer = dagcbor::async_encode(&pointer, store).await.unwrap();
        let decoded_pointer =
            dagcbor::decode::<Pointer<String, i32, Sha3_256>>(encoded_pointer.as_ref()).unwrap();

        assert_eq!(pointer, decoded_pointer);
    }
}
