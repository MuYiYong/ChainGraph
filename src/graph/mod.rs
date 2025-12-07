//! 图核心模块
//!
//! 定义顶点、边和图的核心数据结构

mod edge;
mod graph;
mod index;
mod vertex;

pub use edge::{Edge, EdgeId};
pub use graph::Graph;
pub use index::{EdgeIndex, VertexIndex};
pub use vertex::{Vertex, VertexId};
