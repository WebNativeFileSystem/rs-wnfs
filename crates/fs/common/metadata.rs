//! File system metadata.

use std::str::FromStr;

use anyhow::Result;
use chrono::{DateTime, Utc};
use libipld::Ipld;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// The different types a UnixFS can be.
///
/// See <https://docs.ipfs.io/concepts/file-systems/#unix-file-system-unixfs>
#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum UnixFsNodeKind {
    Raw,
    File,
    Dir,
    Metadata,
    SymLink,
    HAMTShard,
}

/// Mode represents the Unix permissions for a UnixFS node.
///
/// See
/// - <https://docs.ipfs.io/concepts/file-systems/#unix-file-system-unixfs>
/// - <https://en.wikipedia.org/wiki/File-system_permissions#Numeric_notation>
#[derive(Debug, Clone, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum UnixFsMode {
    NoPermissions = 0,
    OwnerReadWriteExecute = 700,
    OwnerGroupReadWriteExecute = 770,
    AllReadWriteExecute = 777,
    AllExecute = 111,
    AllWrite = 222,
    AllWriteExecute = 333,
    AllRead = 444,
    AllReadExecute = 555,
    AllReadWrite = 666,
    OwnerReadWriteExecuteGroupRead = 740,
    OwnerReadWriteExecuteGroupOthersReadExecute = 755,
    OwnerReadWriteGroupOthersRead = 644,
}

/// The metadata of a node in the UnixFS file system.
///
/// See <https://docs.ipfs.io/concepts/file-systems/#unix-file-system-unixfs>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnixFsMetadata {
    pub created: i64,
    pub modified: i64,
    pub mode: UnixFsMode,
    pub kind: UnixFsNodeKind,
}

/// The metadata of a node on the WNFS file system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub unix_fs: UnixFsMetadata,
    pub version: Version,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl Metadata {
    /// Creates a new metadata representing a UnixFS node.
    pub fn new(time: DateTime<Utc>, kind: UnixFsNodeKind) -> Self {
        let mode =
            if matches!(kind, UnixFsNodeKind::Dir) || matches!(kind, UnixFsNodeKind::HAMTShard) {
                UnixFsMode::OwnerReadWriteGroupOthersRead
            } else {
                UnixFsMode::OwnerReadWriteExecuteGroupOthersReadExecute
            };

        let time = time.timestamp();

        Self {
            unix_fs: UnixFsMetadata {
                created: time,
                modified: time,
                mode,
                kind,
            },
            version: Version::new(1, 0, 0),
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self.unix_fs.kind, UnixFsNodeKind::File)
    }
}

impl TryFrom<&Ipld> for Metadata {
    type Error = String;

    fn try_from(ipld: &Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::Map(map) => {
                let unix_fs = map.get("unix_fs").ok_or("Missing unix_fs")?.try_into()?;

                let version = match map.get("version").ok_or("Missing version")? {
                    Ipld::String(v) => Version::from_str(v).map_err(|e| e.to_string())?,
                    _ => return Err("version is not a string".into()),
                };

                Ok(Metadata { unix_fs, version })
            }
            other => Err(format!("Expected `Ipld::Map` got {:#?}", other)),
        }
    }
}

impl TryFrom<&Ipld> for UnixFsMetadata {
    type Error = String;

    fn try_from(ipld: &Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::Map(map) => {
                let created = match map.get("created").ok_or("Missing created")? {
                    Ipld::Integer(i) => *i as i64,
                    _ => return Err("`created` is not an integer".into()),
                };

                let modified = match map.get("modified").ok_or("Missing modified")? {
                    Ipld::Integer(i) => *i as i64,
                    _ => return Err("`modified` is not an integer".into()),
                };

                let mode = map.get("mode").ok_or("Missing mode")?.try_into()?;
                let kind = map.get("kind").ok_or("Missing kind")?.try_into()?;

                Ok(UnixFsMetadata {
                    created,
                    modified,
                    mode,
                    kind,
                })
            }
            other => Err(format!("Expected `Ipld::Map` got {:#?}", other)),
        }
    }
}

impl TryFrom<&Ipld> for UnixFsMode {
    type Error = String;

    fn try_from(ipld: &Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::Integer(i) => UnixFsMode::try_from(*i as u32),
            other => Err(format!("Expected `Ipld::Integer` got {:#?}", other)),
        }
    }
}

impl TryFrom<u32> for UnixFsMode {
    type Error = String;

    fn try_from(num: u32) -> Result<Self, Self::Error> {
        Ok(match num {
            0 => UnixFsMode::NoPermissions,
            700 => UnixFsMode::OwnerReadWriteExecute,
            770 => UnixFsMode::OwnerGroupReadWriteExecute,
            777 => UnixFsMode::AllReadWriteExecute,
            111 => UnixFsMode::AllExecute,
            222 => UnixFsMode::AllWrite,
            333 => UnixFsMode::AllWriteExecute,
            444 => UnixFsMode::AllRead,
            555 => UnixFsMode::AllReadExecute,
            666 => UnixFsMode::AllReadWrite,
            740 => UnixFsMode::OwnerReadWriteExecuteGroupRead,
            755 => UnixFsMode::OwnerReadWriteExecuteGroupOthersReadExecute,
            644 => UnixFsMode::OwnerReadWriteGroupOthersRead,
            _ => return Err(format!("Unknown UnixFsMode: {}", num)),
        })
    }
}

impl TryFrom<&Ipld> for UnixFsNodeKind {
    type Error = String;

    fn try_from(ipld: &Ipld) -> Result<Self, Self::Error> {
        match ipld {
            Ipld::String(s) => UnixFsNodeKind::try_from(s.as_str()),
            other => Err(format!("Expected `Ipld::Integer` got {:#?}", other)),
        }
    }
}

impl TryFrom<&str> for UnixFsNodeKind {
    type Error = String;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        Ok(match name.to_lowercase().as_str() {
            "file" => UnixFsNodeKind::File,
            "dir" => UnixFsNodeKind::Dir,
            "hamt-shard" => UnixFsNodeKind::HAMTShard,
            _ => return Err(format!("Unknown UnixFsNodeKind: {}", name)),
        })
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod metadata_tests {
    use chrono::Utc;

    use crate::{dagcbor, Metadata, UnixFsNodeKind};

    #[async_std::test]
    async fn metadata_can_encode_decode_as_cbor() {
        let metadata = Metadata::new(Utc::now(), UnixFsNodeKind::File);

        let encoded_metadata = dagcbor::encode(&metadata).unwrap();
        let decoded_metadata = dagcbor::decode::<Metadata>(encoded_metadata.as_ref()).unwrap();

        assert_eq!(metadata, decoded_metadata);
    }
}
