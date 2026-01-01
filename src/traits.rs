use crate::index::{EdgeIndex, NodeIndex, PortIndex};

// Change these to trait aliases when the feature is stabilized
// https://github.com/rust-lang/rust/issues/55628
pub trait DataType<T>: Clone + AsRef<T> {}
impl<U, T> DataType<T> for U where U: Clone + AsRef<T> {}

pub trait DataContainer<T: Clone> {
    fn get_data(&self) -> T;

    fn get_data_by_ref(&self) -> &T;

    fn set_data(&mut self, data: T);
}

pub trait GraphLike<DG, DN, DP, DE>: DataContainer<DG>
where
    DG: Clone,
    DN: Clone,
    DP: Clone,
    DE: Clone,
{
    fn add_node(&mut self, node: impl NodeLike<DN, DP>) -> NodeIndex;

    fn remove_node(&mut self, node_idx: NodeIndex) -> impl NodeLike<DN, DP>;

    fn connect_ports(
        &mut self,
        origin_port_idx: PortIndex,
        terminus_port_indes: PortIndex,
    ) -> EdgeIndex;
}

pub trait NodeLike<DN, DP>: DataContainer<DN>
where
    DN: Clone,
    DP: Clone,
{
    fn add_port(&mut self, port: impl PortLike<DP>) -> PortIndex;
}

pub trait PortLike<DP: Clone>: DataContainer<DP> {}

pub trait EdgeLike<DE: Clone>: DataContainer<DE> {
    fn connect_origin(&mut self, origin_port_idx: PortIndex);

    fn connect_terminus(&mut self, terminus_port_idx: PortIndex);
}
