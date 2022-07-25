//! Public node system in-memory representation.

use std::rc::Rc;

use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use libipld::{Cid, Ipld};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::{PublicDirectory, PublicFile};
use crate::{
    common::BlockStore, AsyncSerialize, FsError, Id, Metadata, ReferenceableStore, UnixFsNodeKind,
};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// A node in a WNFS public file system. This can either be a file or a directory.
///
/// PublicNode is serialized as enum.
#[derive(Debug, Clone)]
pub enum PublicNode {
    File(Rc<PublicFile>),
    Dir(Rc<PublicDirectory>),
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PublicNode {
    /// Creates node with updated modified time.
    pub fn update_mtime(&self, time: DateTime<Utc>) -> Self {
        match self {
            Self::File(file) => {
                let mut file = (**file).clone();
                file.metadata.unix_fs.modified = time.timestamp();
                Self::File(Rc::new(file))
            }
            Self::Dir(dir) => {
                let mut dir = (**dir).clone();
                dir.metadata.unix_fs.modified = time.timestamp();
                Self::Dir(Rc::new(dir))
            }
        }
    }

    /// Creates node with updated previous pointer value.
    pub fn update_previous(&self, cid: Option<Cid>) -> Self {
        match self {
            Self::File(file) => {
                let mut file = (**file).clone();
                file.previous = cid;
                Self::File(Rc::new(file))
            }
            Self::Dir(dir) => {
                let mut dir = (**dir).clone();
                dir.previous = cid;
                Self::Dir(Rc::new(dir))
            }
        }
    }

    /// Gets previous ancestor of a node.
    pub fn get_previous(&self) -> Option<Cid> {
        match self {
            Self::File(file) => file.get_previous(),
            Self::Dir(dir) => dir.get_previous(),
        }
    }

    /// Casts a node to a directory.
    ///
    /// # Panics
    ///
    /// Panics if the node is not a directory.
    pub fn as_dir(&self) -> Result<Rc<PublicDirectory>> {
        Ok(match self {
            Self::Dir(dir) => Rc::clone(dir),
            _ => bail!(FsError::NotADirectory),
        })
    }

    /// Casts a node to a file.
    ///
    /// # Panics
    ///
    /// Panics if the node is not a file.
    pub fn as_file(&self) -> Result<Rc<PublicFile>> {
        Ok(match self {
            Self::File(file) => Rc::clone(file),
            _ => bail!(FsError::NotAFile),
        })
    }

    /// Stores a WNFS node as block(s) in chosen block store.
    #[inline]
    pub async fn store<B: BlockStore>(&self, store: &mut B) -> Result<Cid> {
        Ok(match self {
            Self::File(file) => file.store(store).await?,
            Self::Dir(dir) => dir.store(store).await?,
        })
    }

    /// Returns true if underlying node is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Dir(_))
    }

    /// Gets the node kind.
    pub fn kind(&self) -> UnixFsNodeKind {
        match self {
            Self::File(_) => UnixFsNodeKind::File,
            Self::Dir(_) => UnixFsNodeKind::Dir,
        }
    }
}

impl Id for PublicNode {
    fn get_id(&self) -> String {
        match self {
            PublicNode::File(file) => file.get_id(),
            PublicNode::Dir(dir) => dir.get_id(),
        }
    }
}

impl PartialEq for PublicNode {
    fn eq(&self, other: &PublicNode) -> bool {
        match (self, other) {
            (Self::File(self_file), Self::File(other_file)) => {
                Rc::ptr_eq(self_file, other_file) || self_file == other_file
            }
            (Self::Dir(self_dir), Self::Dir(other_dir)) => {
                Rc::ptr_eq(self_dir, other_dir) || self_dir == other_dir
            }
            _ => false,
        }
    }
}

impl<'de> Deserialize<'de> for PublicNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ipld::deserialize(deserializer).and_then(|ipld| ipld.try_into().map_err(de::Error::custom))
    }
}

impl TryFrom<Ipld> for PublicNode {
    type Error = String;

    fn try_from(ipld: Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::Map(map) => {
                let metadata: Metadata = map
                    .get("metadata")
                    .ok_or("Missing metadata field")?
                    .try_into()?;

                Ok(if metadata.is_file() {
                    PublicNode::from(
                        PublicFile::deserialize(Ipld::Map(map)).map_err(|e| e.to_string())?,
                    )
                } else {
                    PublicNode::from(
                        PublicDirectory::deserialize(Ipld::Map(map)).map_err(|e| e.to_string())?,
                    )
                })
            }
            other => Err(format!("Expected `Ipld::Map` got {:#?}", other)),
        }
    }
}

impl From<PublicFile> for PublicNode {
    fn from(file: PublicFile) -> Self {
        Self::File(Rc::new(file))
    }
}

impl From<PublicDirectory> for PublicNode {
    fn from(dir: PublicDirectory) -> Self {
        Self::Dir(Rc::new(dir))
    }
}

/// Implements async deserialization for serde serializable types.
#[async_trait(?Send)]
impl AsyncSerialize for PublicNode {
    type StoreRef = Cid;

    async fn async_serialize<S, RS>(&self, serializer: S, store: &mut RS) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        RS: ReferenceableStore<Ref = Self::StoreRef> + ?Sized,
    {
        match self {
            Self::File(file) => file.serialize(serializer),
            Self::Dir(dir) => dir.async_serialize(serializer, store).await,
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod public_node_tests {
    use std::rc::Rc;

    use chrono::Utc;
    use libipld::Cid;

    use crate::{
        dagcbor,
        public::{PublicDirectory, PublicFile, PublicNode},
        MemoryBlockStore,
    };

    #[async_std::test]
    async fn serialized_public_file_can_be_deserialized() {
        let store = &mut MemoryBlockStore::default();
        let original_node_file =
            PublicNode::File(Rc::new(PublicFile::new(Utc::now(), Cid::default())));

        let serialized_node_file = dagcbor::async_encode(&original_node_file, store)
            .await
            .unwrap();

        let deserialized_node_file: PublicNode =
            dagcbor::decode(serialized_node_file.as_ref()).unwrap();

        assert_eq!(deserialized_node_file, original_node_file);
    }

    #[async_std::test]
    async fn serialized_public_directory_can_be_deserialized() {
        let store = &mut MemoryBlockStore::default();
        let original_node_dir = PublicNode::Dir(Rc::new(PublicDirectory::new(Utc::now())));

        let serialized_node_dir = dagcbor::async_encode(&original_node_dir, store)
            .await
            .unwrap();

        let deserialized_node_dir: PublicNode =
            dagcbor::decode(serialized_node_dir.as_ref()).unwrap();

        assert_eq!(deserialized_node_dir, original_node_dir);
    }
}
