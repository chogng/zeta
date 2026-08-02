//! Backend-neutral native UI framework.
//!
//! `zui` owns declarative element layout, immutable scene composition, inspection metadata,
//! geometry, paint primitives, text layout, and the presentation-only component contract. It does
//! not own reusable product components or a graphics backend.

mod component;
mod element;
mod font;
mod geometry;
mod icon;
mod image;
mod inspection;
mod layout;
mod paint;
#[doc(hidden)]
pub mod renderer_support;
mod scene;
mod text_input;
mod text_layout;

pub use component::Component;
pub use element::{
    ComponentElement, ComputedElement, Element, ElementDirection, ElementLength, ElementStyle,
};
pub use font::{FontCatalog, FontCatalogError};
pub use geometry::{CornerRadii, Edges, Point, Rect, Size};
pub use icon::PaintIcon;
pub use image::{ImageData, ImageDataError, ImageId, PaintImage};
pub use inspection::{InspectionFrame, InspectionNode, InspectionNodeId};
pub use layout::{
    GridLayout, GridLeafLayout, GridNode, GridPane, GridSashLayout, GridSplitLayout,
    SplitViewLayout, SplitViewLayoutPriority, SplitViewOrientation, SplitViewPane, SplitViewResize,
    SplitViewResizeSnapshot, SplitViewSashLayout,
};
pub use paint::{Border, BoxShadow, Color, PaintRect};
pub use scene::{
    FontFamily, FontStyle, FontWeight, SceneBatch, TextBlock, TextBlockWrap, TextSpan, TextStyle,
    UiScene,
};
pub use text_input::{
    CaretBlinkAdvance, CaretBlinkController, CaretVisibility, TextInput, TextInputCommand,
    TextInputCompositionCursor, TextInputCompositionEvent, TextInputLayout, TextInputLayoutEngine,
    TextInputLayoutStyle, TextInputSelectionMode,
};
pub use text_layout::{TextLayout, TextLayoutEngine, TextLayoutWidth};
