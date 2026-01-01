use std::hash::Hash;
use std::ops::Index;

type DefaultIndex = u16;

pub trait IndexType: Copy + Default + Hash + Ord {
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
pub struct NodeIndex<Ix = DefaultIndex>(pub Ix);
impl IndexType for NodeIndex {
    fn new(x: usize) -> Self {
        NodeIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PortIndex<Ix = DefaultIndex>(pub Ix);
impl IndexType for PortIndex {
    fn new(x: usize) -> Self {
        PortIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EdgeIndex<Ix = DefaultIndex>(pub Ix);
impl IndexType for EdgeIndex {
    fn new(x: usize) -> Self {
        EdgeIndex(IndexType::new(x))
    }

    fn index(&self) -> usize {
        self.0.index()
    }
}
