//! Native UI scene and rendering infrastructure.

mod components;
mod font;
mod geometry;
mod icon;
mod icon_renderer;
mod paint;
mod rect_renderer;
mod renderer;
mod scene;
mod text_input;

pub use components::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarSeparatorStyle,
    ActionBarStyle, Button, ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle,
    Component, IconLabel, IconLabelStyle, InputBox, InputBoxState, InputBoxStateColors,
    InputBoxStyle, Tab, TabBackgrounds, TabList, TabListOrientation, TabListStyle, TabSelection,
    TabState, TabStyle,
};
pub use font::{FontCatalog, FontCatalogError};
pub use geometry::{CornerRadii, Edges, Point, Rect, Size};
pub use icon::PaintIcon;
pub use paint::{Border, Color, PaintRect};
pub use renderer::{UiRenderError, UiRenderer, UiViewport};
pub use scene::{FontFamily, FontStyle, FontWeight, TextBlock, TextStyle, UiScene};
pub use text_input::{
    CaretBlinkAdvance, CaretBlinkController, CaretVisibility, TextInput, TextInputCommand,
    TextInputCompositionCursor, TextInputCompositionEvent, TextInputLayout, TextInputLayoutEngine,
    TextInputLayoutStyle, TextInputSelectionMode,
};
