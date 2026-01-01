use std::ops::Index;
use std::hash::Hash;

mod traits;

type DefaultIndex = u16;

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
    
    next_edge_idx: EdgeIndex,
    next_port_idx: PortIndex,
    next_node_idx: NodeIndex,
}

impl<'a, DN, DP, DE> Graph<DN, DP, DE> {
    
    /// Add a node to the graph and assign it the next available index
    pub fn add_node(&mut self, mut node: Node<DN, DP>) {
        let new_node_idx = self.pop_node_index();
        node.set_index(new_node_idx);
        self.nodes.push(node);
    }
    
    /// Retrieve, return, and then increment the next avaible node index
    fn pop_node_index(&mut self) -> NodeIndex {
        let node_idx = self.next_node_idx.clone();
        self.next_node_idx = NodeIndex::new((node_idx.0 + 1) as usize);
        node_idx
    }
    
    /// Retrieve, return, and then increment the next avaible edge index
    fn pop_edge_index(&mut self) -> EdgeIndex {
        let edge_idx = self.next_edge_idx.clone();
        self.next_edge_idx = EdgeIndex::new((edge_idx.0 + 1) as usize);
        edge_idx
    }
    
    /// Retrieve, return, and then increment the next avaible node index
    fn pop_port_index(&mut self) -> PortIndex {
        let port_idx = self.next_port_idx.clone();
        self.next_port_idx = PortIndex::new((port_idx.0 + 1) as usize);
        port_idx
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
            index: NodeIndex::new(0),
            ports: Vec::new(),
        }
    }
    
    pub fn with_data(mut self, data: DN) -> Node<DN, DP> {
        self.data = Some(data);
        self
    }
    
    pub fn add_port(mut self, port: Port<DP>) {
        self.ports.push(port);
    }
    
    pub fn set_index(&mut self, index: NodeIndex) {
        self.index = index;
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Port<DP> {
    pub data: Option<DP>,
    node: EdgeIndex,
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