use std::{collections::BTreeMap, rc::Rc};

use anyhow::{bail, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serializer};

use super::{HamtStore, INumber, Namefilter, PrivateLink, PrivateNode, PrivateNodeHeader, Key};
use crate::{
    AsyncSerialize, BlockStore, FsError, HashOutput, Id, Metadata, ReferenceableStore,
    UnixFsNodeKind,
};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RatchetKey {
    encrypted: Vec<u8>,
    bare: Option<Key>,
}

#[derive(Debug, Clone)]
pub struct PrivateRef {
    pub(crate) saturated_name_hash: HashOutput, // Sha3-256 hash of saturated namefilter
    pub(crate) content_key: Key,            // A hash or parent skip ratchet.
    pub(crate) ratchet_key: RatchetKey,         // Ratchet key.
}

#[derive(Debug, Clone)]
pub struct PrivateDirectoryContent {
    pub(crate) metadata: Metadata,
    pub(crate) entries: BTreeMap<String, PrivateLink>,
}

#[derive(Debug, Clone)]
pub struct PrivateDirectory {
    pub(crate) header: Option<PrivateNodeHeader>,
    pub(crate) content: PrivateDirectoryContent,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateDirectory {
    pub fn new(
        parent_bare_name: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
        time: DateTime<Utc>,
    ) -> Self {
        Self {
            header: Some(PrivateNodeHeader::new(
                parent_bare_name,
                inumber,
                ratchet_seed,
            )),
            content: PrivateDirectoryContent {
                metadata: Metadata::new(time, UnixFsNodeKind::Dir),
                entries: BTreeMap::new(),
            },
        }
    }

    #[async_recursion(?Send)]
    pub async fn get_node<'a, B: BlockStore>(
        self: &Rc<Self>,
        path_segments: &[String],
        hamt: &HamtStore<'a, B>,
    ) -> Result<Option<PrivateNode>> {
        if path_segments.is_empty() {
            bail!(FsError::InvalidPath);
        }

        match path_segments.split_first().unwrap() {
            (head, &[]) => Ok(self.lookup_node(head, hamt).await?),
            (head, tail) => {
                let node = self.lookup_node(head, hamt).await?;
                if !matches!(node, Some(PrivateNode::Dir(_))) {
                    bail!(FsError::InvalidPath);
                }

                self.get_node(tail, hamt).await
            }
        }
    }

    pub async fn lookup_node<'a, B: BlockStore>(
        self: &Rc<Self>,
        path_segment: &str,
        hamt: &HamtStore<'a, B>,
    ) -> Result<Option<PrivateNode>> {
        Ok(match self.content.entries.get(path_segment) {
            Some(link) => Some(link.resolve_value(hamt).await?.clone()),
            None => None,
        })
    }
}

impl Id for PrivateDirectory {
    fn get_id(&self) -> String {
        format!("{:p}", &self.header)
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_directory_tests {}
