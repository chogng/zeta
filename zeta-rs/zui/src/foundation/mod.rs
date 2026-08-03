//! Dependency-free value types shared by every framework layer.

mod color;
mod geometry;

pub use color::Color;
pub use geometry::CornerRadii;
pub use geometry::Edges;
pub use geometry::Point;
pub use geometry::Rect;
pub use geometry::Size;
