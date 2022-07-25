use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{HashOutput, Id, Metadata, UnixFsNodeKind};

use super::{INumber, Namefilter, PrivateNodeHeader};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateFileContent {
    metadata: Metadata,
    content: Vec<u8>, // Inlined file content. // TODO(appcypher): Support linked file content.
}

#[derive(Debug, Clone)]
pub struct PrivateFile {
    header: Option<PrivateNodeHeader>,
    content: PrivateFileContent,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateFile {
    pub fn new(
        parent_bare_name: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
        time: DateTime<Utc>,
        content: Vec<u8>,
    ) -> Self {
        Self {
            header: Some(PrivateNodeHeader::new(
                parent_bare_name,
                inumber,
                ratchet_seed,
            )),
            content: PrivateFileContent {
                metadata: Metadata::new(time, UnixFsNodeKind::File),
                content,
            },
        }
    }
}

impl Id for PrivateFile {
    fn get_id(&self) -> String {
        format!("{:p}", &self.header)
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_directory_tests {}
