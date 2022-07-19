//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

use crate::Referenceable;

use super::{PrivateRef, PrivateNode};

pub type PrivateLink = Referenceable<PrivateRef, PrivateNode>;
