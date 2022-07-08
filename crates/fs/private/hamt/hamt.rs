use std::{collections::BTreeMap, rc::Rc, str::FromStr};

use anyhow::Result;
use async_trait::async_trait;
use libipld::{serde as ipld_serde, Ipld};
use semver::Version;
use serde::{
    de::{DeserializeOwned, Error as DeError},
    ser::Error as SerError,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{AsyncSerialize, BlockStore};

use super::{Node, HAMT_VERSION};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Hamt<K, V> {
    root: Rc<Node<K, V>>,
    version: Version,
    structure: Structure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Structure {
    HAMT,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<K, V> Hamt<K, V> {
    /// Creates a new `Hamt` with the given root node.
    pub fn with_root(root: Rc<Node<K, V>>) -> Self {
        Self {
            root,
            version: HAMT_VERSION,
            structure: Structure::HAMT,
        }
    }

    /// Converts a HAMT to an IPLD object.
    pub async fn to_ipld<B: BlockStore + ?Sized>(&self, store: &mut B) -> Result<Ipld>
    where
        K: Serialize,
        V: Serialize,
    {
        Ok(Ipld::Map(BTreeMap::from([
            ("root".into(), self.root.to_ipld(store).await?),
            ("version".into(), ipld_serde::to_ipld(&self.version)?),
            ("structure".into(), ipld_serde::to_ipld(&self.structure)?),
        ])))
    }
}

#[async_trait(?Send)]
impl<K, V> AsyncSerialize for Hamt<K, V>
where
    K: Serialize,
    V: Serialize,
{
    async fn async_serialize<S: Serializer, B: BlockStore + ?Sized>(
        &self,
        serializer: S,
        store: &mut B,
    ) -> Result<S::Ok, S::Error> {
        self.to_ipld(store)
            .await
            .map_err(SerError::custom)?
            .serialize(serializer)
    }
}

impl<'de, K, V> Deserialize<'de> for Hamt<K, V>
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

impl<K, V> TryFrom<Ipld> for Hamt<K, V>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    type Error = String;

    fn try_from(ipld: Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::Map(mut map) => {
                let root = Rc::new(
                    Node::<K, V>::deserialize(map.remove("root").ok_or("Missing root")?)
                        .map_err(|e| e.to_string())?,
                );

                let version = match map.get("version").ok_or("Missing version")? {
                    Ipld::String(v) => Version::from_str(v).map_err(|e| e.to_string())?,
                    _ => return Err("`version` is not a string".into()),
                };

                let structure = map
                    .get("structure")
                    .ok_or("Missing structure")?
                    .try_into()?;

                Ok(Self {
                    root,
                    version,
                    structure,
                })
            }
            other => Err(format!("Expected `Ipld::Map`, got {:#?}", other)),
        }
    }
}

impl TryFrom<&Ipld> for Structure {
    type Error = String;

    fn try_from(ipld: &Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::String(s) => Structure::try_from(s.as_str()),
            other => Err(format!("Expected `Ipld::Integer` got {:#?}", other)),
        }
    }
}

impl TryFrom<&str> for Structure {
    type Error = String;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        Ok(match name.to_lowercase().as_str() {
            "hamt" => Structure::HAMT,
            _ => return Err(format!("Unknown Structure: {}", name)),
        })
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod hamt_tests {

    use super::*;
    use crate::{dagcbor, MemoryBlockStore};

    #[async_std::test]
    async fn hamt_can_encode_decode_as_cbor() {
        let store = &mut MemoryBlockStore::default();
        let root = Rc::new(Node::default());
        let hamt: Hamt<String, i32> = Hamt::with_root(root);

        let encoded_hamt = dagcbor::async_encode(&hamt, store).await.unwrap();
        let decoded_hamt = dagcbor::decode::<Hamt<String, i32>>(encoded_hamt.as_ref()).unwrap();

        assert_eq!(hamt, decoded_hamt);
    }
}
