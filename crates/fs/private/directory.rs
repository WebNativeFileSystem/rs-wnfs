use std::{collections::BTreeMap, rc::Rc};

use anyhow::{bail, Result};
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};

use super::{HamtStore, INumber, Key, Namefilter, PrivateLink, PrivateNode, PrivateNodeHeader};
use crate::{BlockStore, FsError, HashOutput, Metadata, UnixFsNodeKind};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PrivateRef {
    pub(crate) saturated_name_hash: HashOutput, // Sha3-256 hash of saturated namefilter
    pub(crate) content_key: Rc<Box<dyn Key>>,   // A hash or parent skip ratchet.
    pub(crate) enc_ratchet_key: Vec<u8>,        // Encrypted ratchet key.
}

#[derive(Debug, Clone)]
pub struct PrivateDirectoryMain {
    pub(crate) metadata: Metadata,
    pub(crate) entries: BTreeMap<String, PrivateLink>,
}

#[derive(Debug, Clone)]
pub struct PrivateDirectory {
    pub(crate) header: PrivateNodeHeader,
    pub(crate) main: PrivateDirectoryMain,
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
            header: PrivateNodeHeader::new(parent_bare_name, inumber, ratchet_seed),
            main: PrivateDirectoryMain {
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
        // Ok(match self.main.entries.get(path_segment) {
        //     Some(link) => Some(link.resolve_value(hamt).await?.clone()),
        //     None => None,
        // })
        todo!()
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_directory_tests {}
