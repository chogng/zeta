//! Native UI scene and rendering infrastructure.

mod font;
mod geometry;
mod icon;
mod icon_renderer;
mod paint;
mod rect_renderer;
mod renderer;
mod scene;

pub use font::{FontCatalog, FontCatalogError};
pub use geometry::{CornerRadii, Edges, Point, Rect, Size};
pub use icon::{PaintIcon, SvgIcon};
pub use paint::{Border, Color, PaintRect};
pub use renderer::{UiRenderError, UiRenderer, UiViewport};
pub use scene::{FontFamily, FontStyle, FontWeight, TextBlock, TextStyle, UiScene};
