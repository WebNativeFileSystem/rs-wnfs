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
        let hash = &H::hash(&key);
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
        let hash = &H::hash(key);
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
        let hash = &H::hash(key);
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
        let shift_amount = HAMT_BITMASK_BIT_SIZE - bit_index;
        let mask = if shift_amount < HAMT_BITMASK_BIT_SIZE {
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
        let bit_index = hashnibbles.try_next()?;
        let value_index = self.get_value_index(bit_index);

        debug!(
            "modify_value: bit_index = {}, value_index = {}",
            bit_index, value_index
        );

        // If the bit is not set yet, insert a new pointer.
        if !self.bitmask[bit_index] {
            let mut node = (**self).clone();

            node.pointers
                .insert(value_index, Pointer::Values(vec![Pair { key, value }]));

            node.bitmask.set(bit_index, true);

            return Ok(Rc::new(node));
        }

        Ok(match &self.pointers[value_index] {
            Pointer::Values(values) => {
                let mut node = (**self).clone();
                let pointers: Pointer<_, _, H> = {
                    let mut values = (*values).clone();
                    if let Some(i) = values.iter().position(|p| p.key == key) {
                        // If the key is already present, update the value.
                        values[i] = Pair::new(key, value);
                        Pointer::Values(values)
                    } else {
                        // Otherwise, insert the new value if bucket is not full. Create new node if it is.
                        if values.len() < HAMT_VALUES_BUCKET_SIZE {
                            // Insert in order of key.
                            let index = values
                                .iter()
                                .position(|p| p.key > key)
                                .unwrap_or(values.len());
                            values.insert(index, Pair::new(key, value));
                            Pointer::Values(values)
                        } else {
                            // If values has reached threshold, we need to create a node link that splits it.
                            let mut sub_node = Rc::new(Node::<K, V, H>::default());
                            let cursor = hashnibbles.get_cursor();
                            for Pair { key, value } in
                                values.into_iter().chain(Some(Pair::new(key, value)))
                            {
                                let hash = &H::hash(&key);
                                let hashnibbles = &mut HashNibbles::with_cursor(hash, cursor);
                                sub_node = sub_node
                                    .modify_value(hashnibbles, key, value, store)
                                    .await?;
                            }
                            Pointer::Link(Link::from(sub_node))
                        }
                    }
                };

                node.pointers[value_index] = pointers;
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
        let bit_index = hashnibbles.try_next()?;

        // If the bit is not set yet, return None.
        if !self.bitmask[bit_index] {
            return Ok((Rc::clone(self), None));
        }

        let value_index = self.get_value_index(bit_index);
        Ok(match &self.pointers[value_index] {
            Pointer::Values(values) => {
                let mut node = (**self).clone();
                let value = if values.len() == 1 {
                    // If there is only one value, we can remove the entire pointer.
                    node.bitmask.set(bit_index, false);
                    match node.pointers.remove(value_index) {
                        Pointer::Values(mut values) => Some(values.pop().unwrap()),
                        _ => unreachable!(),
                    }
                } else {
                    // Otherwise, remove just the value.
                    let mut values = (*values).clone();
                    values.iter().position(|p| p.key == *key).map(|i| {
                        let value = values.remove(i);
                        node.pointers[value_index] = Pointer::Values(values);
                        value
                    })
                };

                (Rc::new(node), value)
            }
            Pointer::Link(link) => {
                let child = Rc::clone(link.resolve_value(store).await?);
                let (child, value) = child.remove_value(hashnibbles, key, store).await?;

                let mut node = (**self).clone();
                if value.is_some() {
                    // If something has been deleted, we attempt toc canonicalize the pointer.
                    if let Some(pointer) =
                        Pointer::Link(Link::from(child)).canonicalize(store).await?
                    {
                        node.pointers[value_index] = pointer;
                    } else {
                        // This is None if the pointer now points to an empty node.
                        // In that case, we remove it from the parent.
                        node.bitmask.set(bit_index, false);
                        node.pointers.remove(value_index);
                    }
                } else {
                    node.pointers[value_index] = Pointer::Link(Link::from(child))
                };

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
    use crate::{dagcbor, HashOutput, MemoryBlockStore};
    use lazy_static::lazy_static;
    use test_log::test;

    fn digest(bytes: &[u8]) -> HashOutput {
        let mut nibbles = [0u8; 32];
        nibbles[..bytes.len()].copy_from_slice(bytes);
        nibbles
    }

    lazy_static! {
        static ref HASH_KV_PAIRS: Vec<(HashOutput, &'static str)> = vec![
            (digest(&[0xE0]), "first"),
            (digest(&[0xE1]), "second"),
            (digest(&[0xE2]), "third"),
            (digest(&[0xE3]), "fourth"),
        ];
    }

    #[derive(Debug, Clone)]
    struct MockHasher;
    impl Hasher for MockHasher {
        fn hash<K: AsRef<[u8]>>(key: &K) -> HashOutput {
            let s = std::str::from_utf8(key.as_ref()).unwrap();
            HASH_KV_PAIRS
                .iter()
                .find(|(_, v)| s == *v)
                .unwrap()
                .0
                .clone()
        }
    }

    #[test(async_std::test)]
    async fn get_value_fetches_deeply_linked_value() {
        let store = &mut MemoryBlockStore::default();

        // Insert 4 values to trigger the creation of a linked node.
        let mut working_node = Rc::new(Node::<String, String, MockHasher>::default());
        for (digest, kv) in HASH_KV_PAIRS.iter() {
            let hashnibbles = &mut HashNibbles::new(&digest);
            working_node = working_node
                .modify_value(hashnibbles, kv.to_string(), kv.to_string(), store)
                .await
                .unwrap();
        }

        // Get the values.
        for (digest, kv) in HASH_KV_PAIRS.iter() {
            let hashnibbles = &mut HashNibbles::new(&digest);
            let value = working_node
                .get_value(hashnibbles, &kv.to_string(), store)
                .await
                .unwrap();

            assert_eq!(value, Some(&Pair::new(kv.to_string(), kv.to_string())));
        }
    }

    #[test(async_std::test)]
    async fn remove_value_canonicalizes_linked_node() {
        let store = &mut MemoryBlockStore::default();

        // Insert 4 values to trigger the creation of a linked node.
        let mut working_node = Rc::new(Node::<String, String, MockHasher>::default());
        for (digest, kv) in HASH_KV_PAIRS.iter() {
            let hashnibbles = &mut HashNibbles::new(&digest);
            working_node = working_node
                .modify_value(hashnibbles, kv.to_string(), kv.to_string(), store)
                .await
                .unwrap();
        }

        assert_eq!(working_node.pointers.len(), 1);

        // Remove the third value.
        let third_hashnibbles = &mut HashNibbles::new(&HASH_KV_PAIRS[2].0);
        working_node = working_node
            .remove_value(third_hashnibbles, &"third".to_string(), store)
            .await
            .unwrap()
            .0;

        // Check that the third value is gone.
        match &working_node.pointers[0] {
            Pointer::Values(values) => {
                assert_eq!(values.len(), 3);
            }
            _ => panic!("Expected values pointer"),
        }

        let value = working_node
            .get_value(third_hashnibbles, &"third".to_string(), store)
            .await
            .unwrap();

        assert!(value.is_none());
    }

    #[test(async_std::test)]
    async fn modify_value_splits_when_bucket_threshold_reached() {
        let store = &mut MemoryBlockStore::default();

        // Insert 3 values into the HAMT.
        let mut working_node = Rc::new(Node::<String, String, MockHasher>::default());
        for (idx, (digest, kv)) in HASH_KV_PAIRS.iter().take(3).enumerate() {
            let kv = kv.to_string();
            let hashnibbles = &mut HashNibbles::new(&digest);
            working_node = working_node
                .modify_value(hashnibbles, kv.clone(), kv.clone(), store)
                .await
                .unwrap();

            match &working_node.pointers[0] {
                Pointer::Values(values) => {
                    assert_eq!(values.len(), idx + 1);
                    assert_eq!(values[idx].key, kv.clone());
                    assert_eq!(values[idx].value, kv.clone());
                }
                _ => panic!("Expected values pointer"),
            }
        }

        // Inserting the fourth value should introduce a link indirection.
        working_node = working_node
            .modify_value(
                &mut HashNibbles::new(&HASH_KV_PAIRS[3].0),
                "fourth".to_string(),
                "fourth".to_string(),
                store,
            )
            .await
            .unwrap();

        match &working_node.pointers[0] {
            Pointer::Link(link) => {
                let node = link.get_value().unwrap();
                assert_eq!(node.bitmask.count_ones(), 4);
                assert_eq!(node.pointers.len(), 4);
            }
            _ => panic!("Expected link pointer"),
        }
    }

    #[test(async_std::test)]
    async fn get_value_index_gets_correct_index() {
        let store = &mut MemoryBlockStore::default();
        let hash_expected_idx_samples = [
            (&[0x00], 0),
            (&[0x20], 1),
            (&[0x10], 1),
            (&[0x30], 3),
            (&[0x50], 4),
            (&[0x60], 5),
            (&[0x70], 6),
            (&[0x40], 4),
            (&[0x80], 8),
            (&[0xA0], 9),
            (&[0xB0], 10),
            (&[0xC0], 11),
            (&[0x90], 9),
            (&[0xE0], 13),
            (&[0xD0], 13),
            (&[0xF0], 15),
        ];

        let mut working_node = Rc::new(Node::<String, String>::default());
        for (hash, expected_idx) in hash_expected_idx_samples.into_iter() {
            let bytes = digest(&hash[..]);
            let hashnibbles = &mut HashNibbles::new(&bytes);

            working_node = working_node
                .modify_value(
                    hashnibbles,
                    expected_idx.to_string(),
                    expected_idx.to_string(),
                    store,
                )
                .await
                .unwrap();

            assert_eq!(
                working_node.pointers[expected_idx],
                Pointer::Values(vec![Pair::new(
                    expected_idx.to_string(),
                    expected_idx.to_string()
                )])
            );
        }
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
