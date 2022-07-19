use std::{collections::BTreeMap, rc::Rc};

use anyhow::{bail, Result};
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};

use super::{
    HamtStore, INumber, Key, Namefilter, PrivateLink, PrivateNode, PrivateNodeHeader,
    PrivateNodeSchema,
};
use crate::{BlockStore, FsError, HashOutput, Metadata, UnixFsNodeKind};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

// TODO(appcypher): Resolvable<PrivateNode, EncryptedData>
pub struct PrivateRef {
    pub(crate) namefilter: Namefilter, // TODO(appcypher): Why was this Hash<Namefilter>?
    pub(crate) content_key: Box<dyn Key>,
    pub(crate) encrypted_revision_key: Vec<u8>, // TODO(appcypher): What is it for?
}

pub struct PrivateDirectoryMain {
    pub(crate) metadata: Metadata,
    pub(crate) entries: BTreeMap<String, PrivateLink>,
}

pub type PrivateDirectory = PrivateNodeSchema<PrivateDirectoryMain>;

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateDirectory {
    pub fn new(
        parent_namefilter: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
        time: DateTime<Utc>,
    ) -> Self {
        Self {
            header: PrivateNodeHeader::new(parent_namefilter, inumber, ratchet_seed),
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
        unimplemented!()
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_directory_tests {}
