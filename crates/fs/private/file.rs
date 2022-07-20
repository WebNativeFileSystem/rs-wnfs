use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{HashOutput, Metadata, UnixFsNodeKind};

use super::{INumber, Namefilter, PrivateNodeHeader};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateFileMain {
    metadata: Metadata,
    content: Vec<u8>, // Inlined file content. // TODO(appcypher): Support linked file content.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateFile {
    header: PrivateNodeHeader,
    main: PrivateFileMain,
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
            header: PrivateNodeHeader::new(parent_bare_name, inumber, ratchet_seed),
            main: PrivateFileMain {
                metadata: Metadata::new(time, UnixFsNodeKind::Dir),
                content,
            },
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_directory_tests {}
