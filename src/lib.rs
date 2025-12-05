use std::ops::Index;
use std::hash::Hash;

type DefaultIndex = u32;

trait IndexType: Copy + Default + Hash + Ord {
    fn new(x: usize) -> Self;
    fn index(&self) -> usize;
}

impl IndexType for usize {
    fn new(x: usize) -> Self {
        x
    }

    fn index(&self) -> usize {
        *self
    }
}

impl IndexType for u32 {
    fn new(x: usize) -> Self {
        x as u32
    }

    fn index(&self) -> usize {
        *self as usize
    }
}

impl IndexType for u16 {
    fn new(x: usize) -> Self {
        x as u16
    }

    fn index(&self) -> usize {
        *self as usize
    }
}

impl IndexType for u8 {
    fn new(x: usize) -> Self {
        x as u8
    }

    fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct NodeIndex<Ix = DefaultIndex>(Ix);
impl IndexType for NodeIndex {
    fn new(x: usize) -> Self {
        NodeIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PortIndex<Ix = DefaultIndex>(Ix);
impl IndexType for PortIndex {
    fn new(x: usize) -> Self {
        PortIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EdgeIndex<Ix = DefaultIndex>(Ix);
impl IndexType for EdgeIndex {
    fn new(x: usize) -> Self {
        EdgeIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Direction {
    Omni,
    In,
    Out,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Graph<DN, DP, DE> {
    nodes: Vec<Node<DN, DP>>,
    edges: Vec<Edge<DE>>,
}

impl<'a, DN, DP, DE> Graph<DN, DP, DE> {
    pub fn add_node(mut self, node: Node<DN, DP>) {
        self.nodes.push(node);
    }
    
    pub fn node_by_idx(&'a self, node_idx: NodeIndex) -> Option<&'a Node<DN, DP>> {
        self.nodes.iter().find(|n| n.index == node_idx)
    }
    
    pub fn edge_by_idx(&'a self, edge_idx: EdgeIndex) -> Option<&'a Edge<DE>> {
        self.edges.iter().find(|n| n.index == edge_idx)
    }
    
    // pub fn port_by_idx(&'a self, port_idx: PortIndex) -> Option<&'a Port<DP>> {
    //     self.nodes.iter().map(|n| n.ports.iter().find(|p| p.index == port_idx))
    // }
    
    pub fn connect_ports(mut self, origin_port_idx: PortIndex, terminus_port_idx: PortIndex) {
        
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Node<DN, DP> {
    pub data: Option<DN>,
    index: NodeIndex,
    ports: Vec<Port<DP>>,
}

impl<DN, DP> Node<DN, DP> {
    pub fn new() -> Node<DN, DP> {
        Node {
            data: None,
            index: todo!(),
            ports: vec![],
        }
    }
    pub fn add_port(mut self, port: Port<DP>) {
        self.ports.push(port);
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Port<DP> {
    pub data: Option<DP>,
    node: NodeIndex,
    index: PortIndex,
    direction: Direction,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Edge<DE> {
    pub data: Option<DE>,
    index: EdgeIndex,
    origin: PortIndex,
    terminus: PortIndex,
}

impl<DE> Edge<DE> {
    // pub fn origin_port(&self, graph: Graph<_, _, _>) -> &Port<_> {

    // }
}

// trait NodeType<DP> {
//     fn node_idx(&self) -> impl IndexType;
//     fn ports(&self) -> Vec<&Port<DP>>;
// }

// trait PortType {
//     fn port_idx(&self) -> impl IndexType;
// }

// trait EdgeType {
//     fn edge_index(&self) -> impl IndexType;
//     fn origin_idx(&self) -> impl IndexType;
//     fn terminus_idx(&self) -> impl IndexType;
// }