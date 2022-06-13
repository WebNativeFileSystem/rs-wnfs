//! Public node system in-memory representation.

use std::{rc::Rc};

use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use libipld::{Cid};
use serde::{Deserialize, Serializer};

use super::{PublicDirectory, PublicFile};
use crate::{common::BlockStore, AsyncSerialize, FsError, Id, UnixFsNodeKind};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// A node in a WNFS public file system. This can either be a file or a directory.
///
/// PublicNode is serialized and deserialized as [untagged][1] enum.
///
/// [1]: https://serde.rs/enum-representations.html#untagged
#[derive(Debug, Clone, Deserialize)]
// #[serde(untagged)]
pub enum PublicNode {
    File(Rc<PublicFile>),
    Dir(Rc<PublicDirectory>),
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PublicNode {
    /// Create node with updated modified time.
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

    /// Create node with updated previous pointer value.
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

    /// Get previous ancestor of a node.
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

// impl<'de> Deserialize<'de> for PublicNode {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // We try to deserialize as a file first, if there is an error, we try it as a directory.
//         if let Ok(file) = PublicFile::deserialize(deserializer) {
//             Ok(Self::File(Rc::new(file)))
//         } else {
//             let dir = PublicDirectory::deserialize(deserializer)?;
//             Ok(Self::Dir(Rc::new(dir)))
//         }
//         // TODO(appcypher): Implement visitor for deserialization.
//     }
// }

/// Implements async deserialization for serde serializable types.
#[async_trait(?Send)]
impl AsyncSerialize for PublicNode {
    async fn async_serialize<S: Serializer, B: BlockStore + ?Sized>(
        &self,
        serializer: S,
        store: &mut B,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::File(file) => file.async_serialize(serializer, store).await,
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

        println!("bytes = {:02x?}", serialized_node_file);

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

        println!("bytes = {:02x?}", serialized_node_dir);

        let deserialized_node_dir: PublicNode =
            dagcbor::decode(serialized_node_dir.as_ref()).unwrap();

        assert_eq!(deserialized_node_dir, original_node_dir);
    }
}
