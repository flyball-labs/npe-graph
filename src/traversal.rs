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
pub struct Link {
    pub source: PortId,
    pub edge: EdgeId,
    pub dest: PortId,
}

impl Link {
    fn new(source: PortId, edge: EdgeId, dest: PortId) -> Self {
        Self { source, edge, dest }
    }

    /// Emit a new link from a two `PortId`s and an `EdgeId`,
    /// checking that they're connected in the `Graph`
    fn new_checked<N, P, E>(
        graph: &Graph<N, P, E>,
        source: PortId,
        edge: EdgeId,
        dest: PortId,
    ) -> Result<Link, LinkError> {
        if !graph
            .port_edges(source.clone())
            .collect::<Vec<EdgeId>>()
            .contains(&edge)
            || !graph
                .port_edges(dest.clone())
                .collect::<Vec<EdgeId>>()
                .contains(&edge)
        {
            Err(LinkError::BadLink)
        } else if graph.port_node(dest) == graph.port_node(source) {
            // No jumper edges allowed
            return Err(LinkError::JumperEdge);
        } else {
            Ok(Link::new(source, edge, dest))
        }
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
    RevisitsInterior { link: Link, at: NodeId },
}

impl OpenCycle {}

/// A typestate pattern is used for cycles.
/// This is the open cycle construct; used for
/// cycles still being evaluated by a graph walk that
/// haven't yet been found to be closed.
pub struct OpenCycle(Vec<Link>);

impl OpenCycle {
    /// Create a new empty cycle
    fn new() -> Self {
        Self(vec![])
    }

    /// Return the links
    fn links(&self) -> &[Link] {
        &self.0
    }

    /// Check if the cycle is new/empty
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// Push a new link into the `Cycle`, checking that the last
    /// destination node is the new source node
    fn try_extend<N, P, E>(&mut self, graph: &Graph<N, P, E>, link: Link) -> Result<(), LinkError> {
        // Lookup some ports and nodes on this cycle
        if !self.is_empty() {
            // If the cycle is not empty then run some checks
            let last_cycle_port = self.last_port().expect("the cycle is checked non-empty");
            let last_cycle_node = graph
                .port_node(*last_cycle_port)
                .expect("links in the cycle have been checked good");

            let source_link_node = graph.port_node(link.source).ok_or(LinkError::BadLink)?;

            if source_link_node != last_cycle_node {
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
        let last_port = self.last_port().ok_or(LinkError::EmptyCycle)?;

        let last_node = graph.port_node(*last_port);
        let first_node = graph.port_node(self.links()[0].source);

        if first_node != last_node {
            return Err(LinkError::OpenCycle);
        }
        Ok(ClosedCycle { 0: self.0 })
    }

    fn classify<N, P, E>(&self, graph: &Graph<N, P, E>, edge: EdgeId) -> Result<Step, LinkError> {
        // Get the two endpoints of the edge but we don't yet know which is
        // the source and which is the destination
        let (link_port_a, link_port_b) = graph.edge_endpoints(edge).ok_or(LinkError::BadLink)?;
        let link_node_a = graph.port_node(link_port_a).ok_or(LinkError::BadLink)?;
        let link_node_b = graph.port_node(link_port_b).ok_or(LinkError::BadLink)?;

        // Check if the passed in edge is incident and error if not
        let last_cycle_port = self.last_port().ok_or(LinkError::EmptyCycle)?;
        let last_cycle_node = graph
            .port_node(*last_cycle_port)
            .expect("cycle nodes are good");

        // Determine if either, none, or both of the ends of the `Edge`
        // interface with the final node in the graph
        let (link, dest_link_node) = match (
            link_node_a == last_cycle_node,
            link_node_b == last_cycle_node,
        ) {
            (true, true) => return Err(LinkError::JumperEdge),
            (false, false) => return Err(LinkError::NonAdjacentNodes),
            (true, false) => {
                // link_a is the source
                let link = Link::new(link_port_a, edge, link_port_b);
                let dest_link_node = graph.port_node(link_port_b).ok_or(LinkError::BadLink)?;
                (link, dest_link_node)
            }
            (false, true) => {
                // link_b is the source
                let link = Link::new(link_port_b, edge, link_port_a);
                let dest_link_node = graph.port_node(link_port_a).ok_or(LinkError::BadLink)?;
                (link, dest_link_node)
            }
        };

        // Check if this link closes the cycle
        let first_node = graph
            .port_node(self.links()[0].source)
            .expect("cycle nodes are good");
        if dest_link_node == first_node {
            return Ok(Step::Closes(link));
        }

        // Check if this link revisits an existing node
        // Since a well-formed Cycle contains each NodeId twice (once at source
        // and once at dest) this check only has to look at one of them. This
        // also skips checking the initial Node which means it can't throw a
        // false positive for a closed cycle (even though that's checked above)
        if self
            .links()
            .iter()
            .map(|l| {
                graph
                    .port_node(l.dest)
                    .ok_or(LinkError::CycleNodeNotInGraph)
            })
            .collect::<Result<Vec<NodeId>, LinkError>>()?
            .contains(&dest_link_node)
        {
            return Ok(Step::RevisitsInterior {
                link,
                at: dest_link_node,
            });
        }
        Ok(Step::Extends(link))
    }

    /// Return the last `PortId` in the cycle
    fn last_port(&self) -> Option<&PortId> {
        if self.is_empty() {
            None
        } else {
            Some(&self.links()[self.0.len() - 1].dest)
        }
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
    fn as_node_list<N, P, E>(&self, graph: &Graph<N, P, E>) -> Result<Vec<NodeId>, LinkError> {
        self.0
            .iter()
            .map(|l| l.link_nodes(graph).1.ok_or(LinkError::CycleNodeNotInGraph))
            .collect::<Result<Vec<NodeId>, LinkError>>()
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
            if self.node(node).is_some_and(|n_data| predicate(n_data)) {
                found.push(node)
            }

            self.neighbors(node).into_iter().for_each(|ne| {
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

            if self.node(node).is_some_and(|n_data| predicate(n_data)) {
                found.push(node)
            }

            self.neighbors(node).into_iter().for_each(|ne| {
                if !visited.contains(&ne) {
                    stack.push(ne);
                }
            })
        }

        found
    }

    /// Cutset rank of the component graph, or the number of edges in a
    /// spanning tree
    pub fn cutset_rank(&self) -> usize {
        self.node_count() - 1
    }

    /// Detect a cycle in the graph.
    fn detect_cycles(&self) -> Vec<ClosedCycle> {
        todo!();
        vec![]
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
        intra_predicate: impl Fn(&N, &P, &P) -> bool,
        inter_predicate: impl Fn(&P, &P, &E) -> bool,
    ) -> Vec<ClosedCycle> {
        todo!()
        // vec![]
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

    // ── Link::new_checked ─────────────────────────────────────────────────────
    #[test]
    fn link_checked_valid() {
        let (g, t) = triangle();
        let link = Link::new_checked(&g, t.a_out, t.ab, t.b_in).unwrap();
        assert_eq!(link.source, t.a_out);
        assert_eq!(link.edge, t.ab);
        assert_eq!(link.dest, t.b_in);
    }

    #[test]
    fn link_checked_non_incident_is_bad() {
        let (g, t) = triangle();
        // edge `bc` is not incident to a_out / b_in
        assert!(matches!(
            Link::new_checked(&g, t.a_out, t.bc, t.b_in),
            Err(LinkError::BadLink)
        ));
    }

    #[test]
    fn link_checked_jumper_rejected() {
        let mut g: TG = Graph::new();
        let a = node(&mut g, "A");
        let (p1, p2) = (port(&mut g, a, "p1"), port(&mut g, a, "p2"));
        let j = wire(&mut g, p1, p2);
        assert!(matches!(
            Link::new_checked(&g, p1, j, p2),
            Err(LinkError::JumperEdge)
        ));
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
        let mut oc = OpenCycle(vec![Link::new(t.a_out, t.ab, t.b_in)]);
        let next = Link::new(t.b_out, t.bc, t.c_in);
        assert!(oc.try_extend(&g, next).is_ok());
        assert_eq!(oc.last_port(), Some(&t.c_in));
    }

    #[test]
    fn try_extend_non_adjacent_rejected() {
        let (g, t) = triangle();
        let mut oc = OpenCycle(vec![Link::new(t.a_out, t.ab, t.b_in)]);
        // frontier is B; this link sources from C, not B.
        let bad = Link::new(t.c_out, t.ca, t.a_in);
        assert!(matches!(
            oc.try_extend(&g, bad),
            Err(LinkError::NonAdjacentNodes)
        ));
    }

    // ── try_into_closed ───────────────────────────────────────────────────────
    #[test]
    fn close_open_path_errors() {
        let (g, t) = triangle();
        let oc = OpenCycle(vec![
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
        let oc = OpenCycle(vec![
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
        let oc = OpenCycle(vec![Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
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
        let oc = OpenCycle(vec![
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
        let oc = OpenCycle(vec![
            Link::new(t.a_out, t.ab, t.b_in),
            Link::new(t.b_out, t.bc, t.c_in),
        ]);
        match oc.classify(&g, cb).unwrap() {
            Step::RevisitsInterior { at, .. } => assert_eq!(at, t.b),
            other => panic!("expected RevisitsInterior at B"),
        }
    }

    #[test]
    fn classify_non_adjacent() {
        let (g, t) = triangle();
        let oc = OpenCycle(vec![Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
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
        let oc = OpenCycle(vec![Link::new(t.a_out, t.ab, t.b_in)]); // frontier B
        assert!(matches!(oc.classify(&g, jmp), Err(LinkError::JumperEdge)));
    }

    #[test]
    fn classify_on_empty_errors() {
        let (g, t) = triangle();
        let oc = OpenCycle::new();
        assert!(matches!(oc.classify(&g, t.ab), Err(LinkError::EmptyCycle)));
    }

    // ── detection spec (pending implementation) ──────────────────────────────
    // These encode the intended behavior of the stubbed search functions.
    // Flip off `#[ignore]` as each lands.
    #[test]
    #[ignore = "detect_cycles is todo!()"]
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
    #[ignore = "detect_cycles is todo!()"]
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
    #[ignore = "detect_predicated_cycles is todo!()"]
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
