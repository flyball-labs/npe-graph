# npe-graph

`npe-graph` is a graph data structure consisting of three first class objects: nodes, ports, and edges. A node contains ports, each of which can be connected with edges. All three of these can contain their own data structures. Edges are non-directional (directionality can be enforced by the consumer with the data stored on the edges) and non-exclusive (parallel edges are allowed).

This library is designed for representing various engineering schematics in user-facing programs and file formats. Examples of schematics that conform to the node-port-edge data structure would be an electrical schematic in which the components are nodes, the ports are their pins, and the edges are wires or traces. Hydraulic schematics, controls charts, and architecture diagrams are all similar in structure.

## Why a new graph library?

There are some terrific graph libraries in the Rust ecosystem, including classic hits such as `petgraph`. So why create a new graph structure crate?

Most graph libraries are optimized for very large graph structures with two first class objects: nodes and edges. They can store additional information like weights on the edges or attributes on the nodes. But when implementing a port graph one has to either use a ports-as-nodes or objects-as-nodes structure where the node-port relationship is symbolically stored in a parallel data structure. This becomes cumbersome quite quickly and often obsoletes most of the advantage of using something like `petgraph` in the first place.

Furthermore, existing graph structures are optimized for very large but thin graphs. They're terrific for graph traversal like Dijkstra's algorithm or A*, with a heavy emphasis on performance. Millions of node social networks or road network graphs are in their wheelhouse. But they aren't as useful for storing rich data on the graph primitives.

Engineering schematics are usually in the scale of dozens to hundreds of nodes, trivial from a traversal/compute perspective. But the nodes, ports, and edges need to store arbitrarily useful data structures. The primary use case is direct mapping to user interfaces. Serialization to files is the next most important feature. This library is designed for these smaller data structures with convenient APIs for a user-interface context and serialization/deserialization.

## Design
The two high level decisions in `npe_graph` are how to store the internal data structure and what can be put into the graph.

The first question is currently being answered with [`slotmap`](https://crates.io/crates/slotmap). The `Graph<N, P, E>` struct contains only three fields, three `SlotMap`s for the nodes, ports, and edges. This is the simplest reasonable implementation and will likely remain in place unless performance or other issues necessitate a refactor. This simple architecture was settled on after exploration of more reference-based `Rc<RefCell<T>>` type solutions.

The second question is answered with monomorphism. The `Graph<N, P, E>` struct holds three generic types; a datatype for each of the nodes, ports, and edges. This comes with the minor annoyance that wrapper enums will be required to support different data types. But after exploring a dyn-compatible trait based system the polymorphism came at the cost of much more inconvenient limitations, as dyn-compatibility usually does. And an enumeration of the different types of objects stored on the graph is usually necessary for other functions in these projects.

## Status
`npe-graph` is in one possible good state. But the assumptions made in the API design may be correct in isolation yet inconvenient in application. The crate is being used in multiple actively developed engineering tool projects with fast feedback. Once these projects achieve a degree of maturity it may be assumed that `npe-graph` is correct enough but the crate should be considered unstable until then.
