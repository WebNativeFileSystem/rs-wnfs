//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

use crate::Referenceable;

use super::{PrivateNode, PrivateRef};

pub type PrivateLink = Referenceable<PrivateRef, PrivateNode>;
