//! Public node link.

use std::rc::Rc;

use super::{PublicDirectory, PublicFile, PublicNode};
use crate::Link;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// A link to another node in the WNFS public file system. It can be held as a simple serialised CID or as a reference to the node itself.
///
/// The public file system is a DAG so we don't have to worry bout cyclic references.
pub type PublicLink = Link<PublicNode>;

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PublicLink {
    /// Create a new link to a node.
    pub fn new(node: PublicNode) -> Self {
        Link::from(node)
    }

    /// Creates a new directory node link.
    #[inline]
    pub fn with_dir(dir: Rc<PublicDirectory>) -> Self {
        Link::from(PublicNode::Dir(dir))
    }

    /// Creates a new file node link.
    #[inline]
    pub fn with_file(file: Rc<PublicFile>) -> Self {
        Link::from(PublicNode::File(file))
    }
}
