//! Optional `petgraph` interop (feature = "petgraph").
//!
//! When a solver wants classic algorithms (cycle detection on a control-flow
//! diagram, topological order of a block diagram, connectivity checks),
//! project the rich graph down to a thin one. The projection is throwaway:
//! build it, run the algorithm, map the answers back through the `NodeId` /
//! `EdgeId` weights.

use crate::graph::Graph;
use crate::id::{EdgeId, NodeId};
use petgraph::graph::UnGraph;
use std::collections::HashMap;

impl<N, P, E> Graph<N, P, E> {
    /// Collapses ports away: one petgraph node per component, one petgraph
    /// edge per wire (port-level detail is dropped; endpoints become the
    /// owning nodes). Node and edge weights are this graph's IDs so results
    /// map straight back.
    ///
    /// Internal jumpers (an edge between two ports of the same node) become
    /// petgraph self-loops.
    pub fn to_petgraph(&self) -> UnGraph<NodeId, EdgeId> {
        let mut pg = UnGraph::new_undirected();
        let mut map: HashMap<NodeId, petgraph::graph::NodeIndex> = HashMap::new();
        for (id, _) in self.nodes() {
            map.insert(id, pg.add_node(id));
        }
        for (edge, _) in self.all_edges() {
            if let Some((a, b)) = self.edge_nodes(edge) {
                pg.add_edge(map[&a], map[&b], edge);
            }
        }
        pg
    }
}
