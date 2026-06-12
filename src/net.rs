//! Nets: the solver-facing view of connectivity.
//!
//! In a schematic, a *net* is everything that is electrically (or
//! hydraulically) common — possibly many ports joined by many individual
//! wires. The GUI cares about individual edges (each wire has its own route,
//! its own selection state); a solver (MNA, flow network, signal propagation)
//! cares about nets. This module derives the latter from the former.

use crate::graph::Graph;
use crate::id::{EdgeId, PortId};
use std::collections::HashMap;

/// A maximal set of ports connected (transitively) by edges.
///
/// Isolated ports form singleton nets with no edges — solvers usually want to
/// see those too (a floating pin is a modeling fact, often a bug to report).
#[derive(Debug, Clone, Default)]
pub struct Net {
    /// All ports on this net.
    pub ports: Vec<PortId>,
    /// All edges whose endpoints lie on this net.
    pub edges: Vec<EdgeId>,
}

impl<N, P, E> Graph<N, P, E> {
    /// Computes all nets (connected components over ports, treating every
    /// edge as a perfect junction).
    ///
    /// O(ports + edges) with union-find; intended to be recomputed on demand
    /// after edits rather than maintained incrementally — at the scale this
    /// crate targets (hundreds of elements), that's microseconds.
    ///
    /// ```
    /// # use npe_graph::Graph;
    /// let mut g: Graph<(), (), ()> = Graph::new();
    /// let n1 = g.add_node(()); let n2 = g.add_node(()); let n3 = g.add_node(());
    /// let a = g.add_port(n1, ()).unwrap();
    /// let b = g.add_port(n2, ()).unwrap();
    /// let c = g.add_port(n3, ()).unwrap(); // floating
    /// g.connect(a, b, ()).unwrap();
    /// let nets = g.nets();
    /// assert_eq!(nets.len(), 2);                       // {a,b} and {c}
    /// assert!(nets.iter().any(|n| n.ports.len() == 2 && n.edges.len() == 1));
    /// assert!(nets.iter().any(|n| n.ports == vec![c] && n.edges.is_empty()));
    /// ```
    pub fn nets(&self) -> Vec<Net> {
        // Union-find over a dense renumbering of live ports.
        let port_ids: Vec<PortId> = self.all_ports().map(|(id, _)| id).collect();
        let index: HashMap<PortId, usize> =
            port_ids.iter().enumerate().map(|(i, &p)| (p, i)).collect();

        let mut uf = UnionFind::new(port_ids.len());
        for (edge, _) in self.all_edges() {
            let (a, b) = self.edge_endpoints(edge).expect("live edge");
            uf.union(index[&a], index[&b]);
        }

        // Bucket ports and edges by component root.
        let mut by_root: HashMap<usize, Net> = HashMap::new();
        for (i, &port) in port_ids.iter().enumerate() {
            by_root.entry(uf.find(i)).or_default().ports.push(port);
        }
        for (edge, _) in self.all_edges() {
            let (a, _) = self.edge_endpoints(edge).expect("live edge");
            by_root
                .entry(uf.find(index[&a]))
                .or_default()
                .edges
                .push(edge);
        }
        by_root.into_values().collect()
    }
}

/// Minimal union-find with path halving + union by size.
struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]]; // path halving
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        if self.size[ra] < self.size[rb] {
            core::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}
