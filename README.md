# npe-graph
This library is for representing various engineering schematics in user-facing programs and file formats. A schematic that conform to the node-port-edge data structure would be an electrical schematic in which the components are nodes, the ports are their pins, and the edges are wires or traces. Hydraulic schematics, controls charts, and architecture diagrams are all similar in structure.

Most graph libraries are optimized for very large graph structures with two first glass objects: nodes and edges. They can store additional information like weights on the edges or attributes on the nodes. But when implementing a port graph one has to either use a ports-as-nodes or objects-as-nodes stucture where the node-port relationship is symbolically stored in a parallel data structure. This library is designed for smaller data structures with convenient APIs for a user-interface context.

# Status
npe-graph is in API development mode; the correct data structure and access methods are being worked out alongside UI libraries. At this point, dependencies and performance are secondary concerns.