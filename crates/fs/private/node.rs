use async_trait::async_trait;
use libipld::Cid;
use serde::{Deserialize, Serialize, Serializer};
use skip_ratchet::Ratchet;

use crate::{AsyncSerialize, BlockStore, HashOutput};

use super::{Namefilter, PrivateDirectory, PrivateFile};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type INumber = Vec<u8>;

pub type EncryptedPrivateNodeHeader = Vec<u8>;
pub type EncryptedPrivateNode = (EncryptedPrivateNodeHeader, Vec<Cid>); // (header, [main])

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivateNodeHeader {
    pub(crate) bare_name: Namefilter,
    pub(crate) ratchet: Ratchet,
    pub(crate) inumber: INumber,
}

#[derive(Debug, Clone)]
pub enum PrivateNode {
    Dir(PrivateDirectory),
    File(PrivateFile),
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateNodeHeader {
    /// Creates a new PrivateNodeHeader.
    pub fn new(
        parent_bare_name: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
    ) -> Self {
        Self {
            bare_name: {
                let mut namefilter = parent_bare_name.unwrap_or_default();
                namefilter.add(&inumber);
                namefilter
            },
            ratchet: Ratchet::zero(ratchet_seed),
            inumber,
        }
    }
}

// /// Implements async deserialization for serde serializable types.
// #[async_trait(?Send)]
// impl AsyncSerialize for PrivateNode {
//     async fn async_serialize<S: Serializer, B: BlockStore + ?Sized>(
//         &self,
//         serializer: S,
//         store: &mut B,
//     ) -> Result<S::Ok, S::Error> {
//         match self {
//             Self::File(file) => todo!(), // file.serialize(serializer),
//             Self::Dir(dir) => todo!(), // dir.async_serialize(serializer, store).await,
//         }
//     }
// }

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_node_tests {}
