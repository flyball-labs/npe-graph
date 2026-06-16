//! Graph traversal algorithms
#![allow(dead_code, unused_variables)]

use std::collections::HashSet;

use crate::{EdgeId, NodeId, PortId, graph::Graph};

#[derive(Debug)]
enum LinkError {
    NonAdjacentNodes,
    BadLink,
    EmptyCycle,
    OpenCycle,
}

/// An ordered link between two adjacencies in a graph
// type Link = (PortId, EdgeId, PortId);

struct Link(PortId, EdgeId, PortId);

impl Link {
    fn new(source: PortId, edge: EdgeId, dest: PortId) -> Self {
        Self {
            0: source,
            1: edge,
            2: dest,
        }
    }

    /// Emit a new link from a two `PortId`s and an `EdgeId`,
    /// checking that they're connected in the `Graph`
    fn new_checked<N, P, E>(
        graph: &Graph<N, P, E>,
        source: PortId,
        edge: EdgeId,
        dest: PortId,
    ) -> Result<Link, LinkError> {
        if graph
            .port_edges(source.clone())
            .collect::<Vec<EdgeId>>()
            .contains(&edge)
            && graph
                .port_edges(dest.clone())
                .collect::<Vec<EdgeId>>()
                .contains(&edge)
        {
            Ok(Link::new(source, edge, dest))
        } else {
            Err(LinkError::BadLink)
        }
    }

    /// Return the nodes on either end of the `Link`
    fn link_nodes<N, P, E>(&self, graph: &Graph<N, P, E>) -> (Option<NodeId>, Option<NodeId>) {
        (graph.port_node(self.0), graph.port_node(self.2))
    }

    /// Convenience wrapper for readability
    fn source_as_ref(&self) -> &PortId {
        &self.0
    }

    /// Convenience wrapper for readability
    fn edge_as_ref(&self) -> &EdgeId {
        &self.1
    }

    /// Convenience wrapper for readability
    fn dest_as_ref(&self) -> &PortId {
        &self.2
    }
}

/// A typestate pattern is used for cycles.
/// This is the open cycle construct; used for
/// cycles still being evaluated by a graph walk that
/// haven't yet been found to be closed.
struct OpenCycle(Vec<Link>);

impl OpenCycle {
    /// Create a new empty cycle
    fn new() -> Self {
        Self(vec![])
    }

    /// Return the links
    fn links(&self) -> Vec<&Link> {
        self.0.iter().collect()
    }

    /// Check if the cycle is new/empty
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// Push a new link into the `Cycle`, checking that the last
    /// destination node is the new source node
    fn try_extend<N, P, E>(&mut self, graph: &Graph<N, P, E>, link: Link) -> Result<(), LinkError> {
        if !self.is_empty() {
            // There's at least one link in the cycle, run the check
            let last_node = graph.port_node(*self.links()[self.0.len() - 1].dest_as_ref());
            let next_node = graph.port_node(*link.source_as_ref());
            if last_node != next_node {
                return Err(LinkError::NonAdjacentNodes);
            }
        }

        self.0.push(link);
        return Ok(());
    }

    /// Attempt to close the cycle, checking that the last node
    /// is the first node. This doesn't check every single link for
    /// adjacency. If `try_extend()` has been used to build the `OpenCycle`
    /// then this will be unnecessary and needlessly adds to run time.
    fn try_into_closed<N, P, E>(self, graph: &Graph<N, P, E>) -> Result<ClosedCycle, LinkError> {
        if self.is_empty() {
            return Err(LinkError::EmptyCycle);
        } else {
            let first_node = graph.port_node(*self.links()[0].source_as_ref());
            let last_node = graph.port_node(*self.links()[self.0.len() - 1].dest_as_ref());
            if first_node != last_node {
                return Err(LinkError::OpenCycle);
            }
        }

        Ok(ClosedCycle { 0: self.0 })
    }
}

/// This is the version of a cycle that has been found to be a
/// closed loop. This typestate pattern ensures that any consumer
/// that wants to run analysis on closed cycles never needs to check
/// whether the cycle it's holding is closed or open
struct ClosedCycle(Vec<Link>);

impl ClosedCycle {
    /// Convert a `ClosedCycle` (a list of `PortId`-`EdgeId`-`PortId` links)
    /// into a vector of the `NodeId`s that it contains. This is useful
    /// for cycle analysis tasks that require some accumulation or computation
    /// against the nodes themselves.
    fn as_node_list<N, P, E>(&self, graph: &Graph<N, P, E>) -> Vec<NodeId> {
        let mut nodes: Vec<NodeId> = vec![];
        self.0
            .iter()
            .for_each(|l| nodes.push(l.link_nodes(graph).1.unwrap()));
        nodes
    }
}

impl<N, P, E> Graph<N, P, E> {
    /// Number of connected components of the *component graph* (nodes joined
    /// by wires). Isolated nodes each count as one. O(V + E).
    pub fn connected_components(&self) -> usize {
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut count = 0;
        for (start, _) in self.nodes() {
            if seen.insert(start) {
                count += 1;
                let mut stack = vec![start];
                while let Some(n) = stack.pop() {
                    for m in self.neighbors(n) {
                        if seen.insert(m) {
                            stack.push(m);
                        }
                    }
                }
            }
        }
        count
    }

    /// Undirected cycle rank of the component graph: `E - V + C`, the number of
    /// independent loops.
    pub fn cycle_rank(&self) -> usize {
        let e = self.edge_count();
        let v = self.node_count();
        let c = self.connected_components();
        (e + c).saturating_sub(v)
    }

    /// Detect a cycle in the graph.
    fn detect_cycles(&self) -> Vec<ClosedCycle> {
        todo!()
        // vec![]
    }

    /// Detect a cycle in the graph, predicated on filter functions.
    /// This is how one would implement a direcetional cycle search
    /// or filter out non-functional nodes.
    ///
    /// There are two functions for the two modes of traversal in an
    /// NPE graph.
    /// `intra` is the function that runs to determine if the
    /// traversal should continue within a node, from one port to another
    /// port.
    /// `inter` is the function that runs to determine if the
    /// traversal should continue between one port, through the edge, and
    /// to another port.
    fn detect_predicated_cycles(
        &self,
        intra: impl Fn(&N, &P, &P) -> bool,
        inter: impl Fn(&P, &P, &E) -> bool,
    ) -> Vec<ClosedCycle> {
        todo!()
        // vec![]
    }
}
