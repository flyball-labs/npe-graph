use crate::{
    index::{EdgeIndex, IndexType, NodeIndex, PortIndex},
    traits::{self, DataContainer, DataType, GraphLike, NodeLike, PortLike},
};

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Direction {
    Omni,
    In,
    Out,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Graph<DG, DN, DP, DE> {
    data: DG,
    nodes: Vec<Node<DN, DP>>,
    edges: Vec<Edge<DE>>,

    next_edge_idx: EdgeIndex,
    next_port_idx: PortIndex,
    next_node_idx: NodeIndex,
}

impl<DG, DN, DP, DE> DataContainer<DG> for Graph<DG, DN, DP, DE>
where
    DG: DataType<DG>,
    DN: DataType<DN>,
    DP: DataType<DP>,
    DE: DataType<DE>,
{
    fn get_data(&self) -> DG {
        self.data.clone()
    }

    fn get_data_by_ref(&self) -> &DG {
        self.data.as_ref()
    }

    fn set_data(&mut self, data: DG) {
        self.data = data;
    }
}

impl<DG, DN, DP, DE> GraphLike<DG, DN, DP, DE> for Graph<DG, DN, DP, DE>
where
    DG: DataType<DG>,
    DN: DataType<DN>,
    DP: DataType<DP>,
    DE: DataType<DE>,
{
    fn add_node(&mut self, node: impl traits::NodeLike<DN, DP>) -> NodeIndex {
        todo!()
    }

    fn remove_node(&mut self, node_idx: NodeIndex) -> Node<DN, DP> {
        todo!()
    }

    fn connect_ports(
        &mut self,
        origin_port_idx: PortIndex,
        terminus_port_indes: PortIndex,
    ) -> EdgeIndex {
        todo!()
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Node<DN, DP> {
    data: DN,
    index: NodeIndex,
    ports: Vec<Port<DP>>,
}

impl<DN, DP> DataContainer<DN> for Node<DN, DP>
where
    DN: DataType<DN>,
    DP: DataType<DP>,
{
    fn get_data(&self) -> DN {
        self.data.clone()
    }

    fn get_data_by_ref(&self) -> &DN {
        self.data.as_ref()
    }

    fn set_data(&mut self, data: DN) {
        self.data = data;
    }
}

impl<DN, DP> NodeLike<DN, DP> for Node<DN, DP>
where
    DN: DataType<DN>,
    DP: DataType<DP>,
{
    fn add_port(&mut self, port: impl PortLike<DP>) -> PortIndex {
        todo!()
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Port<DP> {
    data: DP,
    node: EdgeIndex,
    index: PortIndex,
    direction: Direction,
}

impl<DP> DataContainer<DP> for Port<DP>
where
    DP: DataType<DP>,
{
    fn get_data(&self) -> DP {
        self.data.clone()
    }

    fn get_data_by_ref(&self) -> &DP {
        self.data.as_ref()
    }

    fn set_data(&mut self, data: DP) {
        self.data = data;
    }
}

impl<DP> PortLike<DP> for Port<DP> where DP: DataType<DP> {}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Edge<DE> {
    data: DE,
    index: EdgeIndex,
    origin: PortIndex,
    terminus: PortIndex,
}

impl<DE> DataContainer<DE> for Edge<DE>
where
    DE: DataType<DE>,
{
    fn get_data(&self) -> DE {
        self.data.clone()
    }

    fn get_data_by_ref(&self) -> &DE {
        self.data.as_ref()
    }

    fn set_data(&mut self, data: DE) {
        self.data = data;
    }
}
