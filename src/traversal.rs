//! Graph traversal algorithms

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{EdgeId, NodeId, PortId, graph::Graph};

#[derive(Debug)]
pub enum LinkError {
    NonAdjacentNodes,
    BadLink,
    EmptyCycle,
    OpenCycle,
    JumperEdge,
    CycleNodeNotInGraph,
}

/// An ordered link between two adjacencies in a graph
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    pub source: PortId,
    pub edge: EdgeId,
    pub dest: PortId,
}

impl Link {
    pub fn new(source: PortId, edge: EdgeId, dest: PortId) -> Self {
        Self { source, edge, dest }
    }

    /// Return the nodes on either end of the `Link`
    fn link_nodes<N, P, E>(&self, graph: &Graph<N, P, E>) -> (Option<NodeId>, Option<NodeId>) {
        (graph.port_node(self.source), graph.port_node(self.dest))
    }
}

/// The classification of the next step in an `OpenCycle` per
/// a candidate edge
enum Step {
    Extends(Link),
    Closes(Link),
    RevisitsInterior(Link),
}

impl Step {
    /// The resolved link regardless of outcome, for predicate gating.
    fn link(&self) -> &Link {
        match self {
            Step::Extends(l) | Step::Closes(l) | Step::RevisitsInterior(l) => l,
        }
    }
}

/// A typestate pattern is used for cycles.
/// This is the open cycle construct; used for
/// cycles still being evaluated by a graph walk that
/// haven't yet been found to be closed.
pub struct OpenCycle(VecDeque<Link>);

impl OpenCycle {
    /// Create a new empty cycle
    fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Return the links
    fn links(&self) -> impl Iterator<Item = &Link> + '_ {
        self.0.iter()
    }

    /// Check if the cycle is new/empty
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    fn first_port(&self) -> Option<&PortId> {
        self.0.front().map(|l| &l.source)
    }

    fn last_port(&self) -> Option<&PortId> {
        self.0.back().map(|l| &l.dest)
    }

    /// Edge of the most recently added link (the edge the frontier was
    /// reached through), for the U-turn guard.
    fn last_edge(&self) -> Option<EdgeId> {
        self.0.back().map(|l| l.edge)
    }

    /// Remove and return the most recently appended link. Used to backtrack a
    /// frontier extension on DFS unwind.
    fn pop_back(&mut self) -> Option<Link> {
        self.0.pop_back()
    }

    /// Push a new link into the `Cycle`, checking that the last
    /// destination node is the new source node
    fn try_extend<N, P, E>(&mut self, graph: &Graph<N, P, E>, link: Link) -> Result<(), LinkError> {
        if self.is_empty() {
            self.0.push_back(link);
            return Ok(());
        }

        let link_src_node = graph.port_node(link.source).ok_or(LinkError::BadLink)?;
        let link_dst_node = graph.port_node(link.dest).ok_or(LinkError::BadLink)?;

        let first_port = self.first_port().ok_or(LinkError::EmptyCycle)?;
        let last_port = self.last_port().ok_or(LinkError::EmptyCycle)?;

        let first_node = graph.port_node(*first_port).expect("cycle is non-empty");
        let last_node = graph.port_node(*last_port).expect("cycle is non-empty");

        if link_src_node == last_node {
            self.0.push_back(link);
        } else if link_dst_node == first_node {
            self.0.push_front(link);
        } else {
            return Err(LinkError::NonAdjacentNodes);
        }

        Ok(())
    }

    /// Attempt to close the cycle, checking that the last node
    /// is the first node. This doesn't check every single link for
    /// adjacency. If `try_extend()` has been used to build the `OpenCycle`
    /// then this will be unnecessary and needlessly adds to run time.
    fn try_into_closed<N, P, E>(self, graph: &Graph<N, P, E>) -> Result<ClosedCycle, LinkError> {
        let last_port = self.last_port().ok_or(LinkError::EmptyCycle)?;
        let first_port = self.first_port().ok_or(LinkError::EmptyCycle)?;

        let last_node = graph.port_node(*last_port);
        let first_node = graph.port_node(*first_port);

        if first_node != last_node {
            return Err(LinkError::OpenCycle);
        }
        Ok(ClosedCycle(self.0.into_iter().collect()))
    }

    fn classify<N, P, E>(&self, graph: &Graph<N, P, E>, edge: EdgeId) -> Result<Step, LinkError> {
        // Frontier = the node at the back of the path. Orient `edge` so its
        // source sits on that node (errors on jumper / non-incident edges).
        let last_cycle_port = self.last_port().ok_or(LinkError::EmptyCycle)?;
        let frontier = graph
            .port_node(*last_cycle_port)
            .expect("cycle nodes are good");
        let (src, dst, dest_node) = graph.orient_at(frontier, edge)?;
        let link = Link::new(src, edge, dst);

        // Closes if the destination is the path's start node.
        let first_port = self.first_port().ok_or(LinkError::EmptyCycle)?;
        let first_node = graph.port_node(*first_port).expect("cycle nodes are good");
        if dest_node == first_node {
            return Ok(Step::Closes(link));
        }

        // Revisits an interior node. A well-formed path holds each interior
        // node as some link's dest, so scanning dests suffices; the start node
        // is only ever a source, so closure can't masquerade as a revisit.
        if self
            .links()
            .map(|l| {
                graph
                    .port_node(l.dest)
                    .ok_or(LinkError::CycleNodeNotInGraph)
            })
            .collect::<Result<Vec<NodeId>, LinkError>>()?
            .contains(&dest_node)
        {
            return Ok(Step::RevisitsInterior(link));
        }
        Ok(Step::Extends(link))
    }
}

/// This is the version of a cycle that has been found to be a
/// closed loop. This typestate pattern ensures that any consumer
/// that wants to run analysis on closed cycles never needs to check
/// whether the cycle it's holding is closed or open
pub struct ClosedCycle(Vec<Link>);

impl ClosedCycle {
    /// Convert a `ClosedCycle` (a list of `PortId`-`EdgeId`-`PortId` links)
    /// into a vector of the `NodeId`s that it contains. This is useful
    /// for cycle analysis tasks that require some accumulation or computation
    /// against the nodes themselves.
    pub fn as_node_list<N, P, E>(&self, graph: &Graph<N, P, E>) -> Result<Vec<NodeId>, LinkError> {
        self.0
            .iter()
            .map(|l| l.link_nodes(graph).1.ok_or(LinkError::CycleNodeNotInGraph))
            .collect::<Result<Vec<NodeId>, LinkError>>()
    }

    /// A simple wrapper function for returning a reference to the vec of
    /// `Links` contained in the `ClosedCycle`
    pub fn as_vec_list(&self) -> &[Link] {
        &self.0
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

    /// Perform a breadth first search on the graph, returning the
    /// nodes whose data matches the `predicate` Fn argument
    pub fn bfs(&self, start: NodeId, predicate: impl Fn(&N) -> bool) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut found: Vec<NodeId> = vec![];
        let mut queue: VecDeque<NodeId> = VecDeque::from([start]);

        visited.insert(start);

        while let Some(node) = queue.pop_front() {
            if self.node(node).is_some_and(&predicate) {
                found.push(node)
            }

            self.neighbors(node).for_each(|ne| {
                if !visited.contains(&ne) {
                    visited.insert(ne);
                    queue.push_back(ne);
                }
            })
        }

        found
    }

    /// Perform a depth first search on the graph, returning the
    /// nodes whose data matches the `predicate` Fn argument
    pub fn dfs(&self, start: NodeId, predicate: impl Fn(&N) -> bool) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut found: Vec<NodeId> = vec![];
        let mut stack: Vec<NodeId> = vec![start];

        while let Some(node) = stack.pop() {
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node);

            if self.node(node).is_some_and(&predicate) {
                found.push(node)
            }

            self.neighbors(node).for_each(|ne| {
                if !visited.contains(&ne) {
                    stack.push(ne);
                }
            })
        }

        found
    }

    /// Detect every fundamental (independent) cycle in the undirected
    /// component graph. Returns exactly [`Graph::cycle_rank`] cycles, *minus*
    /// any jumper self-loops (edges whose two ports share a node), which this
    /// traversal skips because [`Link`]s are defined between distinct nodes.
    ///
    /// Builds a spanning forest with DFS; each edge that closes back onto an
    /// already-visited node yields one fundamental cycle, recovered by walking
    /// the tree path between the back edge's endpoints. O(V + E).
    pub fn find_cycles(&self) -> Vec<ClosedCycle> {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut came_via: HashMap<NodeId, Link> = HashMap::new();
        let mut seen_edges: HashSet<EdgeId> = HashSet::new();
        let mut cycles: Vec<ClosedCycle> = Vec::new();

        for (start, _) in self.nodes() {
            if !visited.contains(&start) {
                self.fundamental_dfs(
                    start,
                    &mut visited,
                    &mut came_via,
                    &mut seen_edges,
                    &mut cycles,
                );
            }
        }
        cycles
    }

    /// Recursive worker for [`Graph::detect_cycles`]. `seen_edges` is the dedup
    /// mechanism: each edge is processed once, so the parent edge is skipped on
    /// the look-back, parallel edges each get their own turn, and a given back
    /// edge is reported from one side only.
    fn fundamental_dfs(
        &self,
        node: NodeId,
        visited: &mut HashSet<NodeId>,
        came_via: &mut HashMap<NodeId, Link>,
        seen_edges: &mut HashSet<EdgeId>,
        cycles: &mut Vec<ClosedCycle>,
    ) {
        visited.insert(node);

        let incident: Vec<EdgeId> = self.node_edges(node).collect();
        for e in incident {
            // First time we touch this edge from either end wins; the rest skip.
            if !seen_edges.insert(e) {
                continue;
            }

            // Orient the edge so `source` sits on the current node; skip
            // jumpers and non-incident edges.
            let (src_port, dst_port, neighbor) = match self.orient_at(node, e) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if !visited.contains(&neighbor) {
                // Tree edge: record how we reached `neighbor` and descend.
                came_via.insert(neighbor, Link::new(src_port, e, dst_port));
                self.fundamental_dfs(neighbor, visited, came_via, seen_edges, cycles);
            } else {
                // Back edge: reconstruct the fundamental cycle. Seed with the
                // back edge (front = node, back = neighbor), then walk parent
                // pointers from `node` up to `neighbor`; each link prepends.
                let mut links = vec![Link::new(src_port, e, dst_port)];
                let mut cursor = node;
                let mut ok = true;
                while cursor != neighbor {
                    match came_via.get(&cursor) {
                        Some(&link) => match self.port_node(link.source) {
                            Some(parent) => {
                                links.push(link);
                                cursor = parent;
                            }
                            None => {
                                ok = false;
                                break;
                            }
                        },
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    if let Ok(cycle) = self.close_link_path(links) {
                        cycles.push(cycle);
                    } else {
                        debug_assert!(false, "fundamental cycle failed to close");
                    }
                }
            }
        }
    }

    /// Detect cycles in the graph, predicated on filter functions.
    /// This is how one would implement a directional cycle search
    /// or filter out non-functional nodes.
    ///
    /// There are two functions for the two modes of traversal in an
    /// NPE graph.
    /// `intra` is the function that runs to determine if the
    /// traversal should continue within a node, from one port (the one
    /// just entered) to another port (a candidate exit).
    /// `inter` is the function that runs to determine if the
    /// traversal should continue between one port, through the edge, and
    /// to another port.
    ///
    /// Because the predicates make traversal directional, this enumerates
    /// *directed simple cycles* rather than fundamental cycles: a DFS is
    /// rooted at every node and discovered cycles are de-duplicated by their
    /// edge set. Worst-case cost is exponential in the number of cycles
    /// (inherent to simple-cycle enumeration), which is fine for the small
    /// schematics this is intended for.
    pub fn find_predicated_cycles(
        &self,
        intra_predicate: impl Fn(&N, &P, &P) -> bool,
        inter_predicate: impl Fn(&P, &P, &E) -> bool,
    ) -> Vec<ClosedCycle> {
        let mut out: Vec<ClosedCycle> = Vec::new();
        let mut seen_cycles: HashSet<Vec<EdgeId>> = HashSet::new();

        for (s, _) in self.nodes() {
            // First hop out of the start node. No up-front intra check here —
            // the single pass-through of `s` is validated at closure, where we
            // know both the entry port and the exit port `p0`.
            let start_ports: Vec<PortId> = self.ports(s).collect();
            for p0 in start_ports {
                let first_edges: Vec<EdgeId> = self.port_edges(p0).collect();
                for e0 in first_edges {
                    let enter = match self.opposite(e0, p0) {
                        Some(x) => x,
                        None => continue,
                    };
                    let m = match self.port_node(enter) {
                        Some(x) => x,
                        None => continue,
                    };
                    if m == s {
                        continue; // jumper / self-loop
                    }
                    if !self.inter_ok(&inter_predicate, p0, enter, e0) {
                        continue;
                    }

                    // Seed an OpenCycle with the first link and walk from `m`.
                    let mut open = OpenCycle::new();
                    if open.try_extend(self, Link::new(p0, e0, enter)).is_err() {
                        continue;
                    }
                    self.predicated_dfs(
                        s,
                        p0,
                        m,
                        enter,
                        &intra_predicate,
                        &inter_predicate,
                        &mut open,
                        &mut seen_cycles,
                        &mut out,
                    );
                }
            }
        }
        out
    }

    /// Recursive worker for [`Graph::detect_predicated_cycles`].
    ///
    /// `s`/`p0` pin the cycle's start node and its first exit port so the
    /// closing step can validate the pass-through of `s`. `node`/`enter` are
    /// the current frontier: the node we're on and the port we arrived through.
    #[allow(clippy::too_many_arguments)]
    fn predicated_dfs<I, J>(
        &self,
        s: NodeId,
        p0: PortId,
        node: NodeId,
        enter: PortId,
        intra: &I,
        inter: &J,
        open: &mut OpenCycle,
        seen_cycles: &mut HashSet<Vec<EdgeId>>,
        out: &mut Vec<ClosedCycle>,
    ) where
        I: Fn(&N, &P, &P) -> bool,
        J: Fn(&P, &P, &E) -> bool,
    {
        // Never immediately re-traverse the edge we arrived on; a U-turn would
        // otherwise close a spurious 2-cycle under a symmetric `inter`.
        let entering_edge = open.last_edge();

        let exit_ports: Vec<PortId> = self.ports(node).collect();
        for exit_p in exit_ports {
            // Pass through `node`: entered via `enter`, leaving via `exit_p`.
            if !self.intra_ok(intra, node, enter, exit_p) {
                continue;
            }
            let edges: Vec<EdgeId> = self.port_edges(exit_p).collect();
            for e in edges {
                if Some(e) == entering_edge {
                    continue;
                }
                // Structural classification against the current frontier.
                let step = match open.classify(self, e) {
                    Ok(step) => step,
                    Err(_) => continue, // jumper / non-adjacent
                };
                let link = *step.link();
                // Gate the crossing with the inter predicate.
                if !self.inter_ok(inter, link.source, link.dest, link.edge) {
                    continue;
                }

                match step {
                    Step::Closes(_) => {
                        // Validate the pass-through of `s`: enter via the
                        // closing link's dest port, leave via `p0`.
                        if self.intra_ok(intra, s, link.dest, p0) {
                            let mut key: Vec<EdgeId> = open
                                .links()
                                .map(|l| l.edge)
                                .chain(std::iter::once(link.edge))
                                .collect();
                            key.sort();
                            if seen_cycles.insert(key) {
                                let mut full: Vec<Link> = open.links().copied().collect();
                                full.push(link);
                                if let Ok(cycle) = self.close_link_path(full) {
                                    out.push(cycle);
                                } else {
                                    debug_assert!(false, "predicated cycle failed to close");
                                }
                            }
                        }
                    }
                    Step::RevisitsInterior { .. } => {
                        // Already on the path (and not the start): skip to keep
                        // the cycle simple.
                    }
                    Step::Extends(_) => {
                        let neighbor = match self.port_node(link.dest) {
                            Some(x) => x,
                            None => continue,
                        };
                        if open.try_extend(self, link).is_err() {
                            continue;
                        }
                        self.predicated_dfs(
                            s,
                            p0,
                            neighbor,
                            link.dest,
                            intra,
                            inter,
                            open,
                            seen_cycles,
                            out,
                        );
                        open.pop_back();
                    }
                }
            }
        }
    }

    /// Build a [`ClosedCycle`] from an ordered chain of links that is known to
    /// form a closed loop. Feeds them through [`OpenCycle::try_extend`] (whose
    /// bidirectional logic handles prepend vs append) and closes.
    fn close_link_path(&self, links: Vec<Link>) -> Result<ClosedCycle, LinkError> {
        let mut open = OpenCycle::new();
        for link in links {
            open.try_extend(self, link)?;
        }
        open.try_into_closed(self)
    }

    /// Orient `edge` relative to `node`: returns the port on `node` (the
    /// source), the port on the far end (the dest), and the neighbor node.
    /// Shared by [`OpenCycle::classify`] and the fundamental-cycle DFS so the
    /// "which end is mine / is this a jumper" logic lives in one place.
    fn orient_at(&self, node: NodeId, edge: EdgeId) -> Result<(PortId, PortId, NodeId), LinkError> {
        let (pa, pb) = self.edge_endpoints(edge).ok_or(LinkError::BadLink)?;
        let na = self.port_node(pa);
        let nb = self.port_node(pb);
        match (na == Some(node), nb == Some(node)) {
            (true, true) => Err(LinkError::JumperEdge),
            (true, false) => Ok((pa, pb, nb.ok_or(LinkError::BadLink)?)),
            (false, true) => Ok((pb, pa, na.ok_or(LinkError::BadLink)?)),
            (false, false) => Err(LinkError::NonAdjacentNodes),
        }
    }

    /// Apply the intra predicate, resolving node/port data. Missing data (a
    /// stale id) reads as "not traversable".
    fn intra_ok<I>(&self, intra: &I, node: NodeId, enter: PortId, exit: PortId) -> bool
    where
        I: Fn(&N, &P, &P) -> bool,
    {
        match (self.node(node), self.port(enter), self.port(exit)) {
            (Some(n), Some(pe), Some(px)) => intra(n, pe, px),
            _ => false,
        }
    }

    /// Apply the inter predicate, resolving port/edge data. Missing data reads
    /// as "not traversable".
    fn inter_ok<J>(&self, inter: &J, from: PortId, to: PortId, edge: EdgeId) -> bool
    where
        J: Fn(&P, &P, &E) -> bool,
    {
        match (self.port(from), self.port(to), self.edge(edge)) {
            (Some(pf), Some(pt), Some(ed)) => inter(pf, pt, ed),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Graph;

    type TG = Graph<&'static str, &'static str, &'static str>;

    // ── builders ────────────────────────────────────────────────────────────
    fn node(g: &mut TG, name: &'static str) -> NodeId {
        g.add_node(name)
    }
    fn port(g: &mut TG, n: NodeId, name: &'static str) -> PortId {
        g.add_port(n, name).unwrap()
    }
    fn wire(g: &mut TG, a: PortId, b: PortId) -> EdgeId {
        g.connect(a, b, "w").unwrap()
    }

    /// Triangle A-B-C. Returns nodes and the three edges, plus the ports so
    /// tests can build links with correct orientation.
    /// Ports: each node has an "in" and "out"; edges chain out->in around.
    struct Triangle {
        a: NodeId,
        b: NodeId,
        c: NodeId,
        a_in: PortId,
        a_out: PortId,
        b_in: PortId,
        b_out: PortId,
        c_in: PortId,
        c_out: PortId,
        ab: EdgeId,
        bc: EdgeId,
        ca: EdgeId,
    }
    fn triangle() -> (TG, Triangle) {
        let mut g: TG = Graph::new();
        let (a, b, c) = (node(&mut g, "A"), node(&mut g, "B"), node(&mut g, "C"));
        let a_in = port(&mut g, a, "a_in");
        let a_out = port(&mut g, a, "a_out");
        let b_in = port(&mut g, b, "b_in");
        let b_out = port(&mut g, b, "b_out");
        let c_in = port(&mut g, c, "c_in");
        let c_out = port(&mut g, c, "c_out");
        let ab = wire(&mut g, a_out, b_in);
        let bc = wire(&mut g, b_out, c_in);
        let ca = wire(&mut g, c_out, a_in);
        (
            g,
            Triangle {
                a,
                b,
                c,
                a_in,
                a_out,
                b_in,
                b_out,
                c_in,
                c_out,
                ab,
                bc,
                ca,
            },
        )
    }

    fn nodeset(mut v: Vec<NodeId>) -> Vec<NodeId> {
        v.sort();
        v.dedup();
        v
    }

    /// Build an `OpenCycle` directly from a list of links, bypassing the
    /// adjacency checks in `try_extend`. Tests use this to set up a known
    /// cycle state; it mirrors the old `OpenCycle(vec![..])` construction
    /// now that the backing store is a `VecDeque`.
    fn open_cycle(links: impl IntoIterator<Item = Link>) -> OpenCycle {
        OpenCycle(links.into_iter().collect())
    }

    // ── cycle_rank / connected_components ────────────────────────────────────
    #[test]
    fn rank_empty_and_singletons() {
        let mut g: TG = Graph::new();
        assert_eq!(g.cycle_rank(), 0);
        assert_eq!(g.connected_components(), 0);
        node(&mut g, "lonely");
        assert_eq!(g.connected_components(), 1);
        assert_eq!(g.cycle_rank(), 0);
    }

    #[test]
    fn rank_single_edge_is_a_tree() {
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (pa, pb) = (port(&mut g, a, "a"), port(&mut g, b, "b"));
        wire(&mut g, pa, pb);
        assert_eq!(g.cycle_rank(), 0);
        assert_eq!(g.connected_components(), 1);
    }

    #[test]
    fn rank_triangle_one_loop() {
        let (g, _) = triangle();
        assert_eq!(g.cycle_rank(), 1);
        assert_eq!(g.connected_components(), 1);
    }

    #[test]
    fn rank_parallel_edges_one_loop() {
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (a1, a2) = (port(&mut g, a, "a1"), port(&mut g, a, "a2"));
        let (b1, b2) = (port(&mut g, b, "b1"), port(&mut g, b, "b2"));
        wire(&mut g, a1, b1);
        wire(&mut g, a2, b2);
        assert_eq!(g.cycle_rank(), 1); // E=2, V=2, C=1 -> 1
    }

    #[test]
    fn rank_jumper_counts_as_loop() {
        let mut g: TG = Graph::new();
        let a = node(&mut g, "A");
        let (p1, p2) = (port(&mut g, a, "p1"), port(&mut g, a, "p2"));
        wire(&mut g, p1, p2); // internal jumper: self-loop in the component graph
        assert_eq!(g.connected_components(), 1);
        assert_eq!(g.cycle_rank(), 1); // E=1, V=1, C=1 -> 1
    }

    #[test]
    fn rank_disjoint_components() {
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (c, d) = (node(&mut g, "C"), node(&mut g, "D"));
        let (pa, pb) = (port(&mut g, a, "a"), port(&mut g, b, "b"));
        wire(&mut g, pa, pb);
        let (pc, pd) = (port(&mut g, c, "c"), port(&mut g, d, "d"));
        wire(&mut g, pc, pd);
        assert_eq!(g.connected_components(), 2);
        assert_eq!(g.cycle_rank(), 0);
    }

    #[test]
    fn rank_figure_eight_two_loops() {
        // Two triangles sharing node A. Rank should be 2.
        let (mut g, t) = triangle();
        let (d, e) = (node(&mut g, "D"), node(&mut g, "E"));
        let a_x = port(&mut g, t.a, "a_x");
        let a_y = port(&mut g, t.a, "a_y");
        let (d1, d2) = (port(&mut g, d, "d1"), port(&mut g, d, "d2"));
        let (e1, e2) = (port(&mut g, e, "e1"), port(&mut g, e, "e2"));
        wire(&mut g, a_x, d1);
        wire(&mut g, d2, e1);
        wire(&mut g, e2, a_y);
        assert_eq!(g.cycle_rank(), 2);
    }

    #[test]
    fn link_nodes_resolves_both_ends() {
        let (g, t) = triangle();
        let link = Link::new(t.a_out, t.ab, t.b_in);
        assert_eq!(link.link_nodes(&g), (Some(t.a), Some(t.b)));
    }

    // ── OpenCycle basics ──────────────────────────────────────────────────────
    #[test]
    fn open_cycle_starts_empty() {
        let oc = OpenCycle::new();
        assert!(oc.is_empty());
        assert_eq!(oc.last_port(), None);
    }

    #[test]
    fn try_extend_on_empty_seeds_the_cycle() {
        // try_extend now seeds the first link into an empty cycle.
        let (g, t) = triangle();
        let mut oc = OpenCycle::new();
        let link = Link::new(t.a_out, t.ab, t.b_in);
        assert!(oc.try_extend(&g, link).is_ok());
        assert!(!oc.is_empty());
        assert_eq!(oc.last_port(), Some(&t.b_in));
    }

    #[test]
    fn build_full_cycle_through_public_api() {
        // End-to-end: new() -> seed -> extend -> extend -> close, no direct
        // OpenCycle(vec![..]) construction.
        let (g, t) = triangle();
        let mut oc = OpenCycle::new();
        oc.try_extend(&g, Link::new(t.a_out, t.ab, t.b_in)).unwrap();
        oc.try_extend(&g, Link::new(t.b_out, t.bc, t.c_in)).unwrap();
        oc.try_extend(&g, Link::new(t.c_out, t.ca, t.a_in)).unwrap();
        let closed = oc.try_into_closed(&g).expect("triangle closes");
        assert_eq!(
            nodeset(closed.as_node_list(&g).unwrap()),
            nodeset(vec![t.a, t.b, t.c])
        );
    }

    #[test]
    fn try_extend_adjacent_succeeds() {
        let (g, t) = triangle();
        // Seed directly (the only way to insert link #1 today).
        let mut oc = open_cycle([Link::new(t.a_out, t.ab, t.b_in)]);
        let next = Link::new(t.b_out, t.bc, t.c_in);
        assert!(oc.try_extend(&g, next).is_ok());
        assert_eq!(oc.last_port(), Some(&t.c_in));
    }

    #[test]
    fn try_extend_non_adjacent_rejected() {
        // With bidirectional try_extend, a link is rejected only when it's
        // adjacent to NEITHER end of the current path. Seed a single-link
        // cycle A->B (front A, back B), then offer a link on a separate D->E
        // edge that touches neither A nor B.
        let (mut g, t) = triangle();
        let (d, e) = (node(&mut g, "D"), node(&mut g, "E"));
        let (d_out, e_in) = (port(&mut g, d, "d_out"), port(&mut g, e, "e_in"));
        let de = wire(&mut g, d_out, e_in);

        let mut oc = open_cycle([Link::new(t.a_out, t.ab, t.b_in)]);
        // link sources from D and ends at E; neither is the front (A) or back (B).
        let bad = Link::new(d_out, de, e_in);
        assert!(matches!(
            oc.try_extend(&g, bad),
            Err(LinkError::NonAdjacentNodes)
        ));
    }

    #[test]
    fn try_extend_front_extends_path() {
        // The mirror of the back-extension case: a link whose dest node equals
        // the cycle's front node should be prepended, not rejected.
        let (g, t) = triangle();
        // Seed with B->C (front B, back C).
        let mut oc = open_cycle([Link::new(t.b_out, t.bc, t.c_in)]);
        // A->B: dest node B == front node, so this prepends.
        let front = Link::new(t.a_out, t.ab, t.b_in);
        assert!(oc.try_extend(&g, front).is_ok());
        // Front is now A, back still C.
        assert_eq!(oc.first_port(), Some(&t.a_out));
        assert_eq!(oc.last_port(), Some(&t.c_in));
    }

    // ── try_into_closed ───────────────────────────────────────────────────────
    #[test]
    fn close_open_path_errors() {
        let (g, t) = triangle();
        let oc = open_cycle([
            Link::new(t.a_out, t.ab, t.b_in),
            Link::new(t.b_out, t.bc, t.c_in),
        ]); // A->B->C, not closed
        assert!(matches!(oc.try_into_closed(&g), Err(LinkError::OpenCycle)));
    }

    #[test]
    fn close_empty_errors() {
        let (g, _) = triangle();
        let oc = OpenCycle::new();
        assert!(matches!(oc.try_into_closed(&g), Err(LinkError::EmptyCycle)));
    }

    #[test]
    fn close_full_triangle_succeeds() {
        let (g, t) = triangle();
        let oc = open_cycle([
            Link::new(t.a_out, t.ab, t.b_in),
            Link::new(t.b_out, t.bc, t.c_in),
            Link::new(t.c_out, t.ca, t.a_in),
        ]);
        let closed = oc.try_into_closed(&g).expect("triangle closes");
        assert_eq!(
            nodeset(closed.as_node_list(&g).unwrap()),
            nodeset(vec![t.a, t.b, t.c])
        );
    }

    // ── classify: all four outcomes + errors ─────────────────────────────────
    #[test]
    fn classify_extends() {
        let (g, t) = triangle();
        let oc = open_cycle([Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
        match oc.classify(&g, t.bc).unwrap() {
            Step::Extends(link) => {
                assert_eq!(link.source, t.b_out);
                assert_eq!(link.dest, t.c_in);
            }
            _ => panic!("expected Extends"),
        }
    }

    #[test]
    fn classify_closes() {
        let (g, t) = triangle();
        let oc = open_cycle([
            Link::new(t.a_out, t.ab, t.b_in),
            Link::new(t.b_out, t.bc, t.c_in),
        ]); // A->B->C, frontier C
        match oc.classify(&g, t.ca).unwrap() {
            Step::Closes(link) => assert_eq!(link.dest, t.a_in),
            _ => panic!("expected Closes"),
        }
    }

    #[test]
    fn classify_revisits_interior() {
        // A->B->C, plus a second edge from C back to B. Classifying it must
        // report RevisitsInterior at B (not Closes, since B isn't the start).
        let (mut g, t) = triangle();
        let b_alt = port(&mut g, t.b, "b_alt");
        let c_alt = port(&mut g, t.c, "c_alt");
        let cb = wire(&mut g, c_alt, b_alt); // C -> B back-edge
        let oc = open_cycle([
            Link::new(t.a_out, t.ab, t.b_in),
            Link::new(t.b_out, t.bc, t.c_in),
        ]);
        match oc.classify(&g, cb).unwrap() {
            Step::RevisitsInterior(link) => assert_eq!(link.dest, b_alt),
            _other => panic!("expected RevisitsInterior at B"),
        }
    }

    #[test]
    fn classify_non_adjacent() {
        let (g, t) = triangle();
        let oc = open_cycle([Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
        // edge `ca` touches C and A, not B.
        assert!(matches!(
            oc.classify(&g, t.ca),
            Err(LinkError::NonAdjacentNodes)
        ));
    }

    #[test]
    fn classify_jumper_at_frontier() {
        let (mut g, t) = triangle();
        // two extra ports on B with an edge between them: a jumper on the frontier
        let bx = port(&mut g, t.b, "bx");
        let by = port(&mut g, t.b, "by");
        let jmp = wire(&mut g, bx, by);
        let oc = open_cycle([Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
        assert!(matches!(oc.classify(&g, jmp), Err(LinkError::JumperEdge)));
    }

    #[test]
    fn classify_on_empty_errors() {
        let (g, t) = triangle();
        let oc = OpenCycle::new();
        assert!(matches!(oc.classify(&g, t.ab), Err(LinkError::EmptyCycle)));
    }

    // ── detection ────────────────────────────────────────────────────────────
    // Behavior of detect_cycles (undirected fundamental cycles) and
    // detect_predicated_cycles (directed simple cycles).
    #[test]
    fn detect_triangle_one_cycle() {
        let (g, t) = triangle();
        let cycles = g.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(
            nodeset(cycles[0].as_node_list(&g).unwrap()),
            nodeset(vec![t.a, t.b, t.c])
        );
    }

    #[test]
    fn detect_tree_no_cycle() {
        let mut g: TG = Graph::new();
        let (a, b, c) = (node(&mut g, "A"), node(&mut g, "B"), node(&mut g, "C"));
        let (pa, pb1) = (port(&mut g, a, "a"), port(&mut g, b, "b1"));
        wire(&mut g, pa, pb1);
        let (pb2, pc) = (port(&mut g, b, "b2"), port(&mut g, c, "c"));
        wire(&mut g, pb2, pc);
        assert_eq!(g.detect_cycles().len(), 0);
    }

    #[test]
    fn detect_parallel_edges_one_cycle() {
        // Two wires between A and B form a single 2-node loop.
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (a1, a2) = (port(&mut g, a, "a1"), port(&mut g, a, "a2"));
        let (b1, b2) = (port(&mut g, b, "b1"), port(&mut g, b, "b2"));
        wire(&mut g, a1, b1);
        wire(&mut g, a2, b2);
        let cycles = g.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(
            nodeset(cycles[0].as_node_list(&g).unwrap()),
            nodeset(vec![a, b])
        );
    }

    #[test]
    fn detect_figure_eight_two_cycles() {
        // Two triangles sharing node A: cycle_rank 2, so two fundamental cycles.
        let (mut g, t) = triangle();
        let (d, e) = (node(&mut g, "D"), node(&mut g, "E"));
        let a_x = port(&mut g, t.a, "a_x");
        let a_y = port(&mut g, t.a, "a_y");
        let (d1, d2) = (port(&mut g, d, "d1"), port(&mut g, d, "d2"));
        let (e1, e2) = (port(&mut g, e, "e1"), port(&mut g, e, "e2"));
        wire(&mut g, a_x, d1);
        wire(&mut g, d2, e1);
        wire(&mut g, e2, a_y);
        assert_eq!(g.detect_cycles().len(), 2);
        assert_eq!(g.detect_cycles().len(), g.cycle_rank());
    }

    #[test]
    fn predicated_no_valid_direction_no_cycle() {
        // Same feedback pair, but inter never permits a crossing.
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (a_in, a_out) = (port(&mut g, a, "in"), port(&mut g, a, "out"));
        let (b_in, b_out) = (port(&mut g, b, "in"), port(&mut g, b, "out"));
        wire(&mut g, a_out, b_in);
        wire(&mut g, b_out, a_in);
        let intra = |_n: &&str, _p: &&str, _q: &&str| true;
        let inter = |_f: &&str, _t: &&str, _e: &&str| false;
        assert_eq!(g.detect_predicated_cycles(intra, inter).len(), 0);
    }

    #[test]
    fn detect_directed_feedback_pair() {
        // A.out -> B.in, B.out -> A.in : a directed 2-cycle under output->input.
        let mut g: TG = Graph::new();
        let (a, b) = (node(&mut g, "A"), node(&mut g, "B"));
        let (a_in, a_out) = (port(&mut g, a, "in"), port(&mut g, a, "out"));
        let (b_in, b_out) = (port(&mut g, b, "in"), port(&mut g, b, "out"));
        wire(&mut g, a_out, b_in);
        wire(&mut g, b_out, a_in);
        let intra = |_n: &&str, _p: &&str, _q: &&str| true;
        let inter = |from: &&str, _to: &&str, _e: &&str| *from == "out";
        let cycles = g.detect_predicated_cycles(intra, inter);
        assert_eq!(cycles.len(), 1);
    }
}
