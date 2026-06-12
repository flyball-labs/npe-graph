//! Stable, copyable identifiers for graph elements.
//!
//! All three are generational keys: 64 bits, `Copy`, hashable, orderable.
//! A removed element's ID is never re-issued for a different element, so IDs
//! are safe to hold in selections, undo logs, and serialized GUI state.

use slotmap::new_key_type;

new_key_type! {
    /// Identifies a node (component). See [`crate::Graph::add_node`].
    pub struct NodeId;
    /// Identifies a port. Ports are owned by exactly one node.
    pub struct PortId;
    /// Identifies an edge between two ports.
    pub struct EdgeId;
}
