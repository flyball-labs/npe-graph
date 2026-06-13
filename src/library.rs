//! Component libraries: define a component (data + pinout) once, stamp it
//! into the graph many times.
//!
//! A [`NodeTemplate`] is anything that can produce node data plus an ordered
//! pinout. It's deliberately **dyn-compatible**, so a GUI palette can hold a
//! heterogeneous library:
//!
//! ```
//! # use npe_graph::{Graph, NodeTemplate};
//! # struct OpAmp; struct Resistor;
//! # impl NodeTemplate<String, String> for OpAmp {
//! #     fn node_data(&self) -> String { "opamp".into() }
//! #     fn port_data(&self) -> Vec<String> { vec!["in+".into(), "in-".into(), "out".into()] }
//! # }
//! # impl NodeTemplate<String, String> for Resistor {
//! #     fn node_data(&self) -> String { "R".into() }
//! #     fn port_data(&self) -> Vec<String> { vec!["a".into(), "b".into()] }
//! # }
//! let palette: Vec<Box<dyn NodeTemplate<String, String>>> =
//!     vec![Box::new(OpAmp), Box::new(Resistor)];
//!
//! let mut g: Graph<String, String, ()> = Graph::new();
//! let (id, pins) = g.instantiate(palette[0].as_ref());
//! assert_eq!(pins.len(), 3);
//! ```
//!
//! Templates take `&self` and produce owned data: a library entry is shared
//! and stamped many times, so instantiation is inherently a "make me a fresh
//! copy" operation. If your `N` needs per-instance state (reference
//! designators like R1, R2, ...), do that in a wrapper around `instantiate`
//! that owns the counter — the template stays pure.

use crate::graph::Graph;
use crate::id::{NodeId, PortId};

/// A reusable component definition: node data plus an ordered pinout.
///
/// Object-safe by design (`Vec<P>`, not `impl Iterator`), so libraries can be
/// `Vec<Box<dyn NodeTemplate<N, P>>>`.
pub trait NodeTemplate<N, P> {
    /// Fresh node data for one instance of this component.
    fn node_data(&self) -> N;

    /// Fresh port data for one instance, in pinout order. The `PortId`s
    /// returned by [`Graph::instantiate`] correspond to this order, index
    /// for index.
    fn port_data(&self) -> Vec<P>;
}

/// The no-trait-needed escape hatch: a plain blueprint struct.
///
/// If your library is data-driven (loaded from RON/JSON at startup) rather
/// than a set of Rust types, just build these. It implements [`NodeTemplate`]
/// by cloning.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeProto<N, P> {
    /// Data for the node itself.
    pub data: N,
    /// Port data in pinout order.
    pub ports: Vec<P>,
}

impl<N: Clone, P: Clone> NodeTemplate<N, P> for NodeProto<N, P> {
    fn node_data(&self) -> N {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<P> {
        self.ports.clone()
    }
}

impl<N, P, E> Graph<N, P, E> {
    /// Stamps one instance of `template` into the graph: adds the node, then
    /// its ports in pinout order.
    ///
    /// Returns the new node and its ports, where `ports[i]` corresponds to
    /// `template.port_data()[i]` — so a GUI symbol or a netlister can address
    /// pins positionally. (The same ordering is also recoverable later via
    /// [`Graph::ports`], which preserves insertion order.)
    ///
    /// Accepts `&dyn NodeTemplate<N, P>` as well as concrete types.
    pub fn instantiate(
        &mut self,
        template: &(impl NodeTemplate<N, P> + ?Sized),
    ) -> (NodeId, Vec<PortId>) {
        let node = self.add_node(template.node_data());
        let ports = template
            .port_data()
            .into_iter()
            .map(|p| {
                self.add_port(node, p)
                    .expect("node was just added; it must exist")
            })
            .collect();
        (node, ports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ic74x00; // quad NAND, abridged pinout

    impl NodeTemplate<&'static str, &'static str> for Ic74x00 {
        fn node_data(&self) -> &'static str {
            "74x00"
        }
        fn port_data(&self) -> Vec<&'static str> {
            vec!["1A", "1B", "1Y", "GND", "VCC"]
        }
    }

    #[test]
    fn instantiate_concrete_and_dyn() {
        let mut g: Graph<&str, &str, ()> = Graph::new();

        // Concrete.
        let (u1, u1_pins) = g.instantiate(&Ic74x00);
        assert_eq!(g[u1], "74x00");
        assert_eq!(u1_pins.len(), 5);
        assert_eq!(g[u1_pins[3]], "GND");

        // Through a heterogeneous palette.
        let palette: Vec<Box<dyn NodeTemplate<&str, &str>>> = vec![
            Box::new(Ic74x00),
            Box::new(NodeProto {
                data: "R",
                ports: vec!["a", "b"],
            }),
        ];
        let (u2, _) = g.instantiate(palette[0].as_ref());
        let (r1, r1_pins) = g.instantiate(palette[1].as_ref());
        assert_eq!(g[u2], "74x00");
        assert_eq!(g[r1], "R");
        assert_eq!(r1_pins.len(), 2);

        // Instances are independent: pinout order preserved per node.
        let order: Vec<_> = g.ports(u1).collect();
        assert_eq!(order, u1_pins);
    }
}
