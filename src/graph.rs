//! The core [`Graph`] container.

use crate::id::{EdgeId, NodeId, PortId};
use slotmap::SlotMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A node-port-edge graph.
///
/// Type parameters are the user payloads:
/// * `N` — per-node (component) data
/// * `P` — per-port (pin / terminal) data
/// * `E` — per-edge (wire / pipe / signal) data
///
/// All structural invariants are maintained internally:
/// * every port belongs to exactly one live node,
/// * every edge references exactly two live ports,
/// * removals cascade (node → its ports → their edges).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "N: Serialize, P: Serialize, E: Serialize",
        deserialize = "N: Deserialize<'de>, P: Deserialize<'de>, E: Deserialize<'de>"
    ))
)]
pub struct Graph<N, P, E> {
    nodes: SlotMap<NodeId, NodeEntry<N>>,
    ports: SlotMap<PortId, PortEntry<P>>,
    edges: SlotMap<EdgeId, EdgeEntry<E>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct NodeEntry<N> {
    data: N,
    /// Ports in insertion order — meaningful for GUIs (pin ordering).
    ports: Vec<PortId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct PortEntry<P> {
    data: P,
    node: NodeId,
    /// Edges incident to this port, insertion order. Parallel edges allowed.
    edges: Vec<EdgeId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct EdgeEntry<E> {
    data: E,
    /// The two endpoints, in the order passed to `connect`.
    ports: [PortId; 2],
}

/// Error returned by [`Graph::add_port`] when the owning node doesn't exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeMissing;

/// Errors returned by [`Graph::connect`] / [`Graph::connect_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectError {
    /// One or both port IDs are stale / unknown.
    PortMissing,
    /// Both endpoints are the same port. (Two *different* ports on the same
    /// node are fine — that's an internal jumper.)
    SelfLoop,
    /// The user predicate passed to [`Graph::connect_with`] returned `false`
    /// (e.g. output→output in a dataflow graph, mismatched pipe diameter...).
    Rejected,
}

impl core::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConnectError::PortMissing => write!(f, "one or both ports do not exist"),
            ConnectError::SelfLoop => write!(f, "cannot connect a port to itself"),
            ConnectError::Rejected => write!(f, "connection rejected by validator"),
        }
    }
}

impl std::error::Error for ConnectError {}

impl<N, P, E> Default for Graph<N, P, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, P, E> Graph<N, P, E> {
    // ───────────────────────────── construction ─────────────────────────────

    /// Creates an empty graph.
    pub fn new() -> Self {
        Graph {
            nodes: SlotMap::with_key(),
            ports: SlotMap::with_key(),
            edges: SlotMap::with_key(),
        }
    }

    // ────────────────────────────── mutation ────────────────────────────────

    /// Adds a node (component) carrying `data`. Starts with no ports.
    pub fn add_node(&mut self, data: N) -> NodeId {
        self.nodes.insert(NodeEntry {
            data,
            ports: Vec::new(),
        })
    }

    /// Adds a port to `node`. Ports are returned by [`Graph::ports`] in the
    /// order they were added.
    pub fn add_port(&mut self, node: NodeId, data: P) -> Result<PortId, NodeMissing> {
        if !self.nodes.contains_key(node) {
            return Err(NodeMissing);
        }
        let port = self.ports.insert(PortEntry {
            data,
            node,
            edges: Vec::new(),
        });
        self.nodes[node].ports.push(port);
        Ok(port)
    }

    /// Connects two ports with an edge carrying `data`.
    ///
    /// Parallel edges between the same pair of ports are allowed (draw two
    /// wires if you like; deduplicate at the GUI layer if you don't).
    pub fn connect(&mut self, a: PortId, b: PortId, data: E) -> Result<EdgeId, ConnectError> {
        self.connect_with(a, b, data, |_, _| true)
    }

    /// Like [`Graph::connect`], but runs `check` on the two ports' data first.
    /// This is the hook for domain rules: direction compatibility in dataflow
    /// graphs, voltage domains, pipe diameters, connector genders, etc.
    ///
    /// ```
    /// # use npe_graph::{Graph, ConnectError};
    /// #[derive(PartialEq)] enum Dir { In, Out }
    /// let mut g: Graph<(), Dir, ()> = Graph::new();
    /// let a = g.add_node(()); let b = g.add_node(());
    /// let out = g.add_port(a, Dir::Out).unwrap();
    /// let inp = g.add_port(b, Dir::In).unwrap();
    /// assert!(g.connect_with(out, inp, (), |x, y| *x == Dir::Out && *y == Dir::In).is_ok());
    /// assert_eq!(
    ///     g.connect_with(inp, out, (), |x, y| *x == Dir::Out && *y == Dir::In),
    ///     Err(ConnectError::Rejected)
    /// );
    /// ```
    pub fn connect_with<F>(
        &mut self,
        a: PortId,
        b: PortId,
        data: E,
        check: F,
    ) -> Result<EdgeId, ConnectError>
    where
        F: FnOnce(&P, &P) -> bool,
    {
        if a == b {
            return Err(ConnectError::SelfLoop);
        }
        let (pa, pb) = match (self.ports.get(a), self.ports.get(b)) {
            (Some(pa), Some(pb)) => (pa, pb),
            _ => return Err(ConnectError::PortMissing),
        };
        if !check(&pa.data, &pb.data) {
            return Err(ConnectError::Rejected);
        }
        let edge = self.edges.insert(EdgeEntry {
            data,
            ports: [a, b],
        });
        self.ports[a].edges.push(edge);
        self.ports[b].edges.push(edge);
        Ok(edge)
    }

    /// Removes an edge, returning its data. Stale IDs return `None`.
    pub fn disconnect(&mut self, edge: EdgeId) -> Option<E> {
        let entry = self.edges.remove(edge)?;
        for p in entry.ports {
            if let Some(port) = self.ports.get_mut(p) {
                port.edges.retain(|&e| e != edge);
            }
        }
        Some(entry.data)
    }

    /// Removes a port, cascading removal of every edge incident to it.
    pub fn remove_port(&mut self, port: PortId) -> Option<P> {
        let entry = self.ports.remove(port)?;
        for edge in entry.edges {
            // The other endpoint still holds a reference; `disconnect` would
            // double-remove `port`, so clean up manually.
            if let Some(e) = self.edges.remove(edge) {
                for p in e.ports {
                    if p != port {
                        if let Some(other) = self.ports.get_mut(p) {
                            other.edges.retain(|&x| x != edge);
                        }
                    }
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(entry.node) {
            node.ports.retain(|&p| p != port);
        }
        Some(entry.data)
    }

    /// Removes a node, cascading removal of all its ports and their edges.
    pub fn remove_node(&mut self, node: NodeId) -> Option<N> {
        let entry = self.nodes.remove(node)?;
        for port in entry.ports {
            // Port entries still exist; reuse the cascade (node lookup inside
            // `remove_port` will miss, which is fine — we already removed it).
            self.remove_port(port);
        }
        Some(entry.data)
    }

    /// Removes everything. All previously issued IDs become stale.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.ports.clear();
        self.edges.clear();
    }

    // ─────────────────────────────── access ─────────────────────────────────

    /// Node data, if the ID is live. Stale IDs return `None` (never panic),
    /// which is what you want when a GUI holds selections across edits.
    pub fn node(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(id).map(|n| &n.data)
    }

    /// Mutable node data.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut N> {
        self.nodes.get_mut(id).map(|n| &mut n.data)
    }

    /// Port data.
    pub fn port(&self, id: PortId) -> Option<&P> {
        self.ports.get(id).map(|p| &p.data)
    }

    /// Mutable port data.
    pub fn port_mut(&mut self, id: PortId) -> Option<&mut P> {
        self.ports.get_mut(id).map(|p| &mut p.data)
    }

    /// Edge data.
    pub fn edge(&self, id: EdgeId) -> Option<&E> {
        self.edges.get(id).map(|e| &e.data)
    }

    /// Mutable edge data.
    pub fn edge_mut(&mut self, id: EdgeId) -> Option<&mut E> {
        self.edges.get_mut(id).map(|e| &mut e.data)
    }

    // ────────────────────────────── topology ────────────────────────────────

    /// The node that owns `port`.
    pub fn port_node(&self, port: PortId) -> Option<NodeId> {
        self.ports.get(port).map(|p| p.node)
    }

    /// The two ports an edge connects, in the order passed to `connect`.
    pub fn edge_endpoints(&self, edge: EdgeId) -> Option<(PortId, PortId)> {
        self.edges.get(edge).map(|e| (e.ports[0], e.ports[1]))
    }

    /// The two *nodes* an edge connects (through its ports).
    pub fn edge_nodes(&self, edge: EdgeId) -> Option<(NodeId, NodeId)> {
        let (a, b) = self.edge_endpoints(edge)?;
        Some((self.port_node(a)?, self.port_node(b)?))
    }

    /// Given an edge and one of its ports, the port at the other end.
    pub fn opposite(&self, edge: EdgeId, port: PortId) -> Option<PortId> {
        let (a, b) = self.edge_endpoints(edge)?;
        if port == a {
            Some(b)
        } else if port == b {
            Some(a)
        } else {
            None
        }
    }

    /// A node's ports, in insertion order. Empty iterator for stale IDs.
    pub fn ports(&self, node: NodeId) -> impl Iterator<Item = PortId> + '_ {
        self.nodes
            .get(node)
            .map(|n| n.ports.as_slice())
            .unwrap_or(&[])
            .iter()
            .copied()
    }

    /// Edges incident to a port.
    pub fn port_edges(&self, port: PortId) -> impl Iterator<Item = EdgeId> + '_ {
        self.ports
            .get(port)
            .map(|p| p.edges.as_slice())
            .unwrap_or(&[])
            .iter()
            .copied()
    }

    /// Edges incident to any port of `node`.
    pub fn node_edges(&self, node: NodeId) -> impl Iterator<Item = EdgeId> + '_ {
        self.ports(node).flat_map(|p| self.port_edges(p))
    }

    /// Neighboring nodes of `node` (one entry per connecting edge — a node
    /// connected by two wires appears twice; `collect` into a set to dedup).
    pub fn neighbors(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.ports(node).flat_map(move |p| {
            self.port_edges(p)
                .filter_map(move |e| self.opposite(e, p).and_then(|q| self.port_node(q)))
        })
    }

    /// Edges directly between ports `a` and `b` (parallel edges possible).
    pub fn edges_between(&self, a: PortId, b: PortId) -> impl Iterator<Item = EdgeId> + '_ {
        self.port_edges(a)
            .filter(move |&e| self.opposite(e, a) == Some(b))
    }

    // ─────────────────────────── whole-graph iter ───────────────────────────

    /// All nodes with their data. Order is deterministic but not insertion
    /// order; sort by your own data if presentation order matters.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &N)> + '_ {
        self.nodes.iter().map(|(id, n)| (id, &n.data))
    }

    /// All ports with their data.
    pub fn all_ports(&self) -> impl Iterator<Item = (PortId, &P)> + '_ {
        self.ports.iter().map(|(id, p)| (id, &p.data))
    }

    /// All edges with their data.
    pub fn all_edges(&self) -> impl Iterator<Item = (EdgeId, &E)> + '_ {
        self.edges.iter().map(|(id, e)| (id, &e.data))
    }

    /// Number of live nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of live ports.
    pub fn port_count(&self) -> usize {
        self.ports.len()
    }

    /// Number of live edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Is this ID live?
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// Is this ID live?
    pub fn contains_port(&self, id: PortId) -> bool {
        self.ports.contains_key(id)
    }

    /// Is this ID live?
    pub fn contains_edge(&self, id: EdgeId) -> bool {
        self.edges.contains_key(id)
    }
}

// Panicking sugar: `&graph[node_id]` etc. Use the `Option` getters when the
// ID might be stale.
impl<N, P, E> core::ops::Index<NodeId> for Graph<N, P, E> {
    type Output = N;
    fn index(&self, id: NodeId) -> &N {
        &self.nodes[id].data
    }
}
impl<N, P, E> core::ops::IndexMut<NodeId> for Graph<N, P, E> {
    fn index_mut(&mut self, id: NodeId) -> &mut N {
        &mut self.nodes[id].data
    }
}
impl<N, P, E> core::ops::Index<PortId> for Graph<N, P, E> {
    type Output = P;
    fn index(&self, id: PortId) -> &P {
        &self.ports[id].data
    }
}
impl<N, P, E> core::ops::IndexMut<PortId> for Graph<N, P, E> {
    fn index_mut(&mut self, id: PortId) -> &mut P {
        &mut self.ports[id].data
    }
}
impl<N, P, E> core::ops::Index<EdgeId> for Graph<N, P, E> {
    type Output = E;
    fn index(&self, id: EdgeId) -> &E {
        &self.edges[id].data
    }
}
impl<N, P, E> core::ops::IndexMut<EdgeId> for Graph<N, P, E> {
    fn index_mut(&mut self, id: EdgeId) -> &mut E {
        &mut self.edges[id].data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc_pair() -> (
        Graph<&'static str, &'static str, &'static str>,
        NodeId,
        PortId,
        PortId,
        EdgeId,
    ) {
        let mut g = Graph::new();
        let r = g.add_node("R1");
        let ra = g.add_port(r, "a").unwrap();
        let rb = g.add_port(r, "b").unwrap();
        let c = g.add_node("C1");
        let cp = g.add_port(c, "+").unwrap();
        let w = g.connect(rb, cp, "w1").unwrap();
        (g, r, ra, rb, w)
    }

    #[test]
    fn cascade_node_removal() {
        let (mut g, r, ra, rb, w) = rc_pair();
        g.remove_node(r);
        assert!(!g.contains_node(r));
        assert!(!g.contains_port(ra));
        assert!(!g.contains_port(rb));
        assert!(!g.contains_edge(w));
        assert_eq!(g.node_count(), 1); // C1 survives
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn cascade_port_removal_cleans_other_endpoint() {
        let (mut g, _r, _ra, rb, w) = rc_pair();
        g.remove_port(rb);
        assert!(!g.contains_edge(w));
        // No port should still list the dead edge.
        for (p, _) in g.all_ports() {
            assert!(g.port_edges(p).all(|e| g.contains_edge(e)));
        }
    }

    #[test]
    fn stale_ids_miss_quietly() {
        let (mut g, r, ra, _rb, w) = rc_pair();
        g.remove_node(r);
        assert_eq!(g.node(r), None);
        assert_eq!(g.port(ra), None);
        assert_eq!(g.edge(w), None);
        assert_eq!(g.ports(r).count(), 0);
    }

    #[test]
    fn self_loop_rejected() {
        let (mut g, _r, ra, _rb, _w) = rc_pair();
        assert_eq!(g.connect(ra, ra, "x"), Err(ConnectError::SelfLoop));
    }
}
