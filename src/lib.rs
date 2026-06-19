//! # npe-graph
//!
//! A **node–port–edge** graph for engineering schematics: circuit diagrams,
//! control-flow / block diagrams, hydraulic and pneumatic schematics.
//!
//! ## The data model
//!
//! ```text
//!   ┌─────────── Node ───────────┐        ┌────────── Node ──────────┐
//!   │  N (component data)        │        │  N                       │
//!   │                            │        │                          │
//!   │  ○ Port (P)  ○ Port (P) ───┼─Edge───┼─ ○ Port (P)   ○ Port (P) │
//!   └────────────────────────────┘  (E)   └──────────────────────────┘
//! ```
//!
//! * A **node** is a component (op-amp, valve, PID block). Carries user data `N`.
//! * A **port** is a connection point *owned by exactly one node* (pin, flange,
//!   signal input). Carries user data `P`. Ports have an identity of their own:
//!   they can be looked up, iterated, and referenced by edges directly.
//! * An **edge** connects two *ports* (never two nodes directly). Carries user
//!   data `E` (wire gauge, pipe diameter, signal type, GUI spline route...).
//!
//! ## Quick example
//!
//! ```
//! use npe_graph::Graph;
//!
//! // N = component, P = pin, E = wire — use your own rich types.
//! let mut g: Graph<&str, &str, &str> = Graph::new();
//!
//! let r1 = g.add_node("R1: resistor 10k");
//! let r1_a = g.add_port(r1, "a").unwrap();
//! let r1_b = g.add_port(r1, "b").unwrap();
//!
//! let c1 = g.add_node("C1: cap 100n");
//! let c1_pos = g.add_port(c1, "+").unwrap();
//!
//! let w = g.connect(r1_b, c1_pos, "wire").unwrap();
//!
//! assert_eq!(g.port_node(r1_b), Some(r1));
//! assert_eq!(g.edge_endpoints(w), Some((r1_b, c1_pos)));
//! assert!(g.neighbors(r1).any(|n| n == c1));
//!
//! // Removing a node cascades: its ports and their edges go too.
//! g.remove_node(c1);
//! assert!(!g.contains_edge(w));
//! assert!(g.contains_port(r1_a)); // unrelated IDs unaffected
//! ```

mod graph;
mod id;
mod library;
mod net;
mod traversal;

#[cfg(feature = "petgraph")]
mod interop;

pub use graph::{ConnectError, Graph, NodeMissing};
pub use id::{EdgeId, NodeId, PortId};
pub use library::{KeyedNodeTemplate, NodeProto, NodeTemplate};
pub use net::Net;
