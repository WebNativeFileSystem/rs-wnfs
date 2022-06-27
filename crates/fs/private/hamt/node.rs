use std::{fmt::Debug, marker::PhantomData, rc::Rc};

use crate::{private::HAMT_VALUES_BUCKET_SIZE, AsyncSerialize, BlockStore, Link};
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use bitvec::array::BitArray;

use libipld::{serde as ipld_serde, Ipld};
use log::debug;
use serde::{
    de::{Deserialize, DeserializeOwned},
    ser::Error as SerError,
    Deserializer, Serialize, Serializer,
};
use sha3::Sha3_256;

use super::{
    error::HamtError,
    hash::{HashNibbles, Hasher},
    Pair, Pointer, HAMT_BITMASK_BIT_SIZE, HAMT_BITMASK_BYTE_SIZE,
};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type BitMaskType = [u8; HAMT_BITMASK_BYTE_SIZE];

#[derive(Debug, Clone)]
pub struct Node<K, V, H = Sha3_256>
where
    H: Hasher,
{
    pub(crate) bitmask: BitArray<BitMaskType>,
    pub(crate) pointers: Vec<Pointer<K, V, H>>,
    hasher: PhantomData<H>,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<K, V, H> Node<K, V, H>
where
    K: DeserializeOwned + Serialize + AsRef<[u8]> + Clone + Eq + PartialOrd + Debug,
    V: DeserializeOwned + Serialize + Clone + Debug,
    H: Hasher + Clone + Debug,
{
    /// Sets a new value at the given key.
    pub async fn set<B: BlockStore>(
        self: Rc<Self>,
        key: K,
        value: V,
        store: &mut B,
    ) -> Result<Rc<Self>> {
        let hash = &Sha3_256::generate_hash(&key);
        debug!("set: hash = {:02x?}", hash);
        self.modify_value(&mut HashNibbles::new(hash), key, value, store)
            .await
    }

    /// Gets the value at the given key.
    pub async fn get<'a, B: BlockStore>(
        self: &'a Rc<Self>,
        key: &K,
        store: &B,
    ) -> Result<Option<&'a V>> {
        let hash = &Sha3_256::generate_hash(key);
        debug!("get: hash = {:02x?}", hash);
        Ok(self
            .get_value(&mut HashNibbles::new(hash), key, store)
            .await?
            .map(|pair| &pair.value))
    }

    /// Removes the value at the given key.
    pub async fn remove<'a, B: BlockStore>(
        self: Rc<Self>,
        key: &K,
        store: &B,
    ) -> Result<(Rc<Self>, Option<V>)> {
        let hash = &Sha3_256::generate_hash(key);
        debug!("remove: hash = {:02x?}", hash);
        self.remove_value(&mut HashNibbles::new(hash), key, store)
            .await
            .map(|(node, pair)| (node, pair.map(|pair| pair.value)))
    }

    /// Checks if the node is empty.
    pub fn is_empty(&self) -> bool {
        self.bitmask.is_empty()
    }

    /// Calculates the value index from the bitmask index.
    pub(super) fn get_value_index(&self, bit_index: usize) -> usize {
        let shift_amount = HAMT_BITMASK_SIZE - bit_index;
        let mask = if shift_amount < HAMT_BITMASK_SIZE {
            let mut tmp = BitArray::<BitMaskType>::new([0xff, 0xff]);
            tmp.shift_left(shift_amount);
            tmp
        } else {
            BitArray::ZERO
        };
        assert_eq!(mask.count_ones(), bit_index);
        (mask & self.bitmask).count_ones()
    }

    #[async_recursion(?Send)]
    pub(super) async fn modify_value<'a, 'b, B: BlockStore>(
        self: &'a Rc<Self>,
        hashnibbles: &'b mut HashNibbles,
        key: K,
        value: V,
        store: &B,
    ) -> Result<Rc<Self>> {
        let bit_index = hashnibbles.next()? as usize;
        debug!("modify_value: bit_index = {}", bit_index);
        let value_index = self.get_value_index(bit_index);

        // If the bit is not set yet, insert a new pointer.
        if !self.bitmask[bit_index] {
            let mut node = (**self).clone();
            node.pointers
                .insert(value_index, Pointer::Values(vec![Pair { key, value }]));
            node.bitmask.set(bit_index, true);
            return Ok(Rc::new(node));
        }

        // Otherwise, we go one level deep.
        Ok(match &self.pointers[value_index] {
            Pointer::Values(values) => {
                let mut node = (**self).clone();
                let mut values = (*values).clone();

                if let Some(i) = values.iter().position(|p| p.key == key) {
                    values[i] = Pair { key, value };
                } else {
                    values.insert(value_index, Pair { key, value });
                }

                node.pointers[value_index] = Pointer::Values(values);
                Rc::new(node)
            }
            Pointer::Link(link) => {
                let child = Rc::clone(link.resolve_value(store).await?);
                let child = child.modify_value(hashnibbles, key, value, store).await?;
                let mut node = (**self).clone();
                node.pointers[value_index] = Pointer::Link(Link::from(child));
                Rc::new(node)
            }
        })
    }

    #[async_recursion(?Send)]
    pub(super) async fn get_value<'a, 'b, B: BlockStore>(
        self: &'a Rc<Self>,
        hashnibbles: &'b mut HashNibbles,
        key: &K,
        store: &B,
    ) -> Result<Option<&'a Pair<K, V>>> {
        let bit_index = hashnibbles.try_next()?;

        // If the bit is not set yet, return None.
        if !self.bitmask[bit_index] {
            return Ok(None);
        }

        // Otherwise, we go one level deep.
        let value_index = self.get_value_index(bit_index);
        match &self.pointers[value_index] {
            Pointer::Values(values) => Ok(values.iter().find(|kv| key.eq(&kv.key))),
            Pointer::Link(link) => {
                let child = link.resolve_value(store).await?;
                child.get_value(hashnibbles, key, store).await
            }
        }
    }

    #[async_recursion(?Send)]
    pub(super) async fn remove_value<'a, 'b, B: BlockStore>(
        self: &'a Rc<Self>,
        hashnibbles: &'b mut HashNibbles,
        key: &K,
        store: &B,
    ) -> Result<(Rc<Self>, Option<Pair<K, V>>)> {
        let bit_index = hashnibbles.next()? as usize;

        // If the bit is not set yet, return None.
        if !self.bitmask[bit_index] {
            return Ok((Rc::clone(self), None));
        }

        // Otherwise, we go one level deep.
        let value_index = self.get_value_index(bit_index);
        Ok(match &self.pointers[value_index] {
            Pointer::Values(values) => {
                let mut node = (**self).clone();
                let mut values = (*values).clone();
                let value = values
                    .iter()
                    .position(|p| &p.key == key)
                    .map(|i| values.remove(i));
                node.pointers[value_index] = Pointer::Values(values);
                (Rc::new(node), value)
            }
            Pointer::Link(link) => {
                let child = Rc::clone(link.resolve_value(store).await?);
                let (child, value) = child.remove_value(hashnibbles, key, store).await?;
                let mut node = (**self).clone();
                node.pointers[value_index] = Pointer::Link(Link::from(child));
                (Rc::new(node), value)
            }
        })
    }
}

impl<K, V, H: Hasher> Node<K, V, H> {
    /// Returns the count of the values in all the values pointer of a node.
    pub fn count_values(self: &Rc<Self>) -> Result<usize> {
        let mut len = 0;
        for i in self.pointers.iter() {
            if let Pointer::Values(values) = i {
                len += values.len();
            } else {
                bail!(HamtError::ValuesPointerExpected);
            }
        }

        Ok(len)
    }

    /// Converts a Node to an IPLD object.
    pub async fn to_ipld<B: BlockStore + ?Sized>(&self, store: &mut B) -> Result<Ipld>
    where
        K: Serialize,
        V: Serialize,
    {
        let bitmask_ipld = ipld_serde::to_ipld(&self.bitmask.as_raw_slice())?;
        let pointers_ipld = {
            let mut tmp = Vec::with_capacity(self.pointers.len());
            for pointer in self.pointers.iter() {
                tmp.push(pointer.to_ipld(store).await?);
            }
            Ipld::List(tmp)
        };

        Ok(Ipld::List(vec![bitmask_ipld, pointers_ipld]))
    }
}

impl<K, V, H: Hasher> Default for Node<K, V, H> {
    fn default() -> Self {
        Node {
            bitmask: BitArray::ZERO,
            pointers: Vec::with_capacity(HAMT_BITMASK_BIT_SIZE),
            hasher: PhantomData,
        }
    }
}

#[async_trait(?Send)]
impl<K, V, H> AsyncSerialize for Node<K, V, H>
where
    K: Serialize,
    V: Serialize,
    H: Hasher,
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

impl<'de, K, V, H> Deserialize<'de> for Node<K, V, H>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
    H: Hasher,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (bitmask, pointers): (BitMaskType, _) = Deserialize::deserialize(deserializer)?;
        Ok(Node {
            bitmask: BitArray::<BitMaskType>::from(bitmask),
            pointers,
            hasher: PhantomData,
        })
    }
}

impl<K, V, H> PartialEq for Node<K, V, H>
where
    K: PartialEq,
    V: PartialEq,
    H: Hasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.bitmask == other.bitmask && self.pointers == other.pointers
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod hamt_node_tests {
    use super::*;
    use crate::{dagcbor, private::hamt::hash::HashOutput, MemoryBlockStore};
    use test_log::test;

    fn nibbles(bytes: &[u8]) -> HashOutput {
        let mut nibbles = [0u8; 32];
        nibbles[..bytes.len()].copy_from_slice(bytes);
        nibbles
    }

    #[test(async_std::test)]
    async fn modify_value_can_change_pointer() {
        let mut store = MemoryBlockStore::default();
        let node = Rc::new(Node::<String, String>::default());

        let bytes = &nibbles(&[0x21, 0x10]);
        let nibbles_1 = &mut HashNibbles::new(bytes);

        let bytes = &nibbles(&[0x21, 0x20]);
        let nibbles_2 = &mut HashNibbles::new(bytes);

        let result_1 = node
            .modify_value(nibbles_1, String::new(), "First".into(), &mut store)
            .await
            .unwrap();

        debug!(
            "result_1: bitmask: {:?}, pointers: {:#?}",
            result_1.bitmask, result_1.pointers
        );

        let result_2 = result_1
            .modify_value(nibbles_2, String::new(), "Second".into(), &mut store)
            .await
            .unwrap();

        debug!(
            "result_2: bitmask: {:?}, pointers: {:#?}",
            result_2.bitmask, result_2.pointers
        );
    }

    #[test(async_std::test)]
    async fn get_value_index_gets_correct_index() {
        // TODO(appcypher): We need the modify_value test working, then we can test if indexes are correct after multiple inserts.
        let node = Rc::new(Node::<String, String>::default());

        let bytes = &nibbles(&[0x25]);
        let nibbles_1 = &mut HashNibbles::new(bytes);
    }

    #[test(async_std::test)]
    async fn node_can_insert_pair_and_retrieve() {
        let mut store = MemoryBlockStore::default();
        let node = Rc::new(Node::<String, (i32, f64)>::default());

        let node = node
            .set("pill".into(), (10, 0.315), &mut store)
            .await
            .unwrap();

        let value = node.get(&"pill".into(), &mut store).await.unwrap().unwrap();

        assert_eq!(value, &(10, 0.315));
    }

    #[test(async_std::test)]
    async fn node_can_encode_decode_as_cbor() {
        let store = &mut MemoryBlockStore::default();
        let node: Rc<Node<String, i32>> = Rc::new(Node::default());

        let node = node.set("James".into(), 4500, store).await.unwrap();
        let node = node.set("Peter".into(), 2000, store).await.unwrap();

        let encoded_node = dagcbor::async_encode(&node, store).await.unwrap();
        let decoded_node = dagcbor::decode::<Node<String, i32>>(encoded_node.as_ref()).unwrap();

        assert_eq!(*node, decoded_node);
    }
}
