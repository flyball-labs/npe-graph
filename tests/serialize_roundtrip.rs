//! Proves the load-bearing property: after a serialize→deserialize round trip,
//! the keys stored *inside* the graph (an edge's port IDs, a port's owning
//! node) still resolve to the correct live elements. If slotmap didn't
//! preserve versioned keys across serde, this is exactly what would break.

#![cfg(feature = "serde")]

use npe_graph::{EdgeId, Graph, NodeId, PortId};

type G = Graph<String, String, String>;

/// A small graph with a deletion, so the surviving keys carry non-trivial
/// generation bits (the removed slot's version was bumped).
fn sample() -> (G, NodeId, PortId, PortId, EdgeId) {
    let mut g: G = Graph::new();

    // Create then remove a node so live slots are not generation 0.
    let scratch = g.add_node("scratch".into());
    g.remove_node(scratch);

    let r = g.add_node("R1".into());
    let ra = g.add_port(r, "a".into()).unwrap();
    let rb = g.add_port(r, "b".into()).unwrap();

    let c = g.add_node("C1".into());
    let cp = g.add_port(c, "+".into()).unwrap();

    let w = g.connect(rb, cp, "wire".into()).unwrap();
    (g, r, ra, rb, w)
}

/// Every structural cross-reference must still hold, and old IDs must still
/// address the same data.
fn assert_intact(g: &G, r: NodeId, ra: PortId, rb: PortId, w: EdgeId) {
    assert_eq!(g.node(r).map(String::as_str), Some("R1"));
    assert_eq!(g.port(ra).map(String::as_str), Some("a"));
    assert_eq!(g.port_node(rb), Some(r)); // port→node ref survived
    let (p, q) = g.edge_endpoints(w).expect("edge survived");
    assert_eq!(p, rb); // edge→port refs survived, same key values
    assert_eq!(
        g.port_node(q).and_then(|n| g.node(n)).map(String::as_str),
        Some("C1")
    );
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 1);
    // Derived net view recomputes correctly post-load.
    assert!(
        g.nets()
            .iter()
            .any(|n| n.ports.len() == 2 && n.edges.len() == 1)
    );
}

#[test]
fn json_roundtrip() {
    let (g, r, ra, rb, w) = sample();
    let s = serde_json::to_string(&g).unwrap();
    let g2: G = serde_json::from_str(&s).unwrap();
    assert_intact(&g2, r, ra, rb, w);
}

#[test]
fn ron_roundtrip() {
    let (g, r, ra, rb, w) = sample();
    let s = ron::ser::to_string(&g).unwrap();
    let g2: G = ron::from_str(&s).unwrap();
    assert_intact(&g2, r, ra, rb, w);
}

#[test]
fn bincode_roundtrip_non_self_describing() {
    // The interesting case: bincode is NOT self-describing. If any inner serde
    // impl relied on `deserialize_any`, this would fail where JSON/RON pass.
    let (g, r, ra, rb, w) = sample();
    let bytes = bincode::serialize(&g).unwrap();
    let g2: G = bincode::deserialize(&bytes).unwrap();
    assert_intact(&g2, r, ra, rb, w);
}

#[test]
fn ids_are_stable_values_across_load() {
    // Not just "resolves to equivalent data" — the actual key value is
    // unchanged, so IDs held *outside* the graph (a GUI selection, an undo
    // entry) remain valid after save/load.
    let (g, _r, _ra, rb, w) = sample();
    let s = serde_json::to_string(&g).unwrap();
    let g2: G = serde_json::from_str(&s).unwrap();
    // rb and w were captured before serialization; they still address the
    // same elements in the freshly loaded graph.
    assert!(g2.contains_port(rb));
    assert!(g2.contains_edge(w));
    assert_eq!(g2.edge_endpoints(w).unwrap().0, rb);
}
