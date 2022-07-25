use std::rc::Rc;

use async_trait::async_trait;
use libipld::{Cid, Ipld};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use skip_ratchet::Ratchet;

use crate::{AsyncSerialize, HashOutput, Id, Metadata, ReferenceableStore};

use super::{Namefilter, PrivateDirectory, PrivateFile, PrivateRef};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type INumber = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivateNodeHeader {
    pub(crate) bare_name: Namefilter,
    pub(crate) ratchet: Ratchet,
    pub(crate) inumber: INumber,
}

#[derive(Debug, Clone)]
pub enum PrivateNode {
    File(Rc<PrivateFile>),
    Dir(Rc<PrivateDirectory>),
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

impl Id for PrivateNode {
    fn get_id(&self) -> String {
        match self {
            PrivateNode::File(file) => file.get_id(),
            PrivateNode::Dir(dir) => dir.get_id(),
        }
    }
}

// TODO(appcypher): Figure how to get rid of the unnecessary contraint.
#[async_trait(?Send)]
impl AsyncSerialize for PrivateNode {
    type StoreRef = PrivateRef;

    async fn async_serialize<S, RS>(
        &self,
        _serializer: S,
        _store: &mut RS,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        RS: ReferenceableStore<Ref = Self::StoreRef> + ?Sized,
    {
        unimplemented!()
    }
}

// TODO(appcypher): Figure how to get rid of the unnecessary contraint.
impl<'de> Deserialize<'de> for PrivateNode {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}

impl From<PrivateFile> for PrivateNode {
    fn from(file: PrivateFile) -> Self {
        Self::File(Rc::new(file))
    }
}

impl From<PrivateDirectory> for PrivateNode {
    fn from(dir: PrivateDirectory) -> Self {
        Self::Dir(Rc::new(dir))
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

mod private_node_tests {}
