use libipld::Cid;
use skip_ratchet::Ratchet;

use crate::HashOutput;

use super::{Namefilter, PrivateDirectory, PrivateFile};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type INumber = Vec<u8>;

pub type EncryptedPrivateNodeHeader = Vec<u8>;
pub type EncryptedPrivateNode = (EncryptedPrivateNodeHeader, Vec<Cid>); // (header, [main])

#[derive(Debug, Clone, PartialEq)]
pub struct PrivateNodeHeader {
    pub(crate) namefilter: Namefilter,
    pub(crate) ratchet: Ratchet,
    pub(crate) inumber: INumber,
}

pub struct PrivateNodeSchema<M> {
    pub(crate) header: PrivateNodeHeader,
    pub(crate) main: M,
}

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
        parent_namefilter: Option<Namefilter>,
        inumber: INumber,
        ratchet_seed: HashOutput,
    ) -> Self {
        Self {
            namefilter: {
                let mut namefilter = parent_namefilter.unwrap_or_default();
                namefilter.add(&inumber);
                namefilter
            },
            ratchet: Ratchet::zero(ratchet_seed),
            inumber,
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_node_tests {}
