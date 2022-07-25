use crate::Referenceable;

use super::{PrivateNode, PrivateRef};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type PrivateLink = Referenceable<PrivateRef, PrivateNode>;
