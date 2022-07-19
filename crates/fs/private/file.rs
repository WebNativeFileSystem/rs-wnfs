use chrono::{DateTime, Utc};
use skip_ratchet::Ratchet;

use crate::{HashOutput, Metadata, UnixFsNodeKind};

use super::{INumber, Namefilter, PrivateNodeHeader, PrivateNodeSchema};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub enum PrivateFileContent {
    Data(Vec<u8>),
    Structure { ratchet: Ratchet, count: u32 }, // TODO(appcypher): I don't fully understand how this works.
}

pub struct PrivateFileMain {
    metadata: Metadata,
    content: PrivateFileContent,
}

pub type PrivateFile = PrivateNodeSchema<PrivateFileMain>;

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateFile {
    pub fn new(
        parent_namefilter: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
        time: DateTime<Utc>,
        content: PrivateFileContent,
    ) -> Self {
        Self {
            header: PrivateNodeHeader::new(parent_namefilter, inumber, ratchet_seed),
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
