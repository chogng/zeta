//! Native UI scene and rendering infrastructure.

mod components;
mod font;
mod geometry;
mod icon;
mod icon_renderer;
mod image;
mod image_renderer;
mod layout;
mod paint;
mod rect_renderer;
mod renderer;
mod scene;
mod text_input;
mod text_layout;

pub use components::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarSeparatorStyle,
    ActionBarStyle, Button, ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle,
    Component, ContextMenu, ContextMenuItem, ContextMenuSelection, ContextMenuStyle, ContextView,
    ContextViewAnchorAlignment, ContextViewAnchorAxis, ContextViewAnchorPosition,
    ContextViewLayout, ContextViewPlacement, ContextViewStyle, Dropdown, DropdownItem,
    DropdownSelection, DropdownStyle, IconLabel, IconLabelStyle, InputBox, InputBoxState,
    InputBoxStateColors, InputBoxStyle, Sash, SashOrientation, SashState, SashStyle, ScrollAxis,
    ScrollCommand, ScrollDelta, ScrollMetrics, ScrollState, ScrollView, ScrollViewStyle,
    ScrollViewport, ScrollbarController, ScrollbarDrag, ScrollbarHit, ScrollbarLayout,
    ScrollbarPart, ScrollbarPointerPresence, ScrollbarPresentation, ScrollbarState, ScrollbarStyle,
    ScrollbarVisibility, SearchBox, SearchBoxStyle, Tab, TabBackgrounds, TabList,
    TabListOrientation, TabListStyle, TabSelection, TabState, TabStyle,
};
pub use font::{FontCatalog, FontCatalogError};
pub use geometry::{CornerRadii, Edges, Point, Rect, Size};
pub use icon::PaintIcon;
pub use image::{ImageData, ImageDataError, ImageId, PaintImage};
pub use layout::{
    GridLayout, GridLeafLayout, GridNode, GridPane, GridSashLayout, GridSplitLayout,
    SplitViewLayout, SplitViewLayoutPriority, SplitViewOrientation, SplitViewPane, SplitViewResize,
    SplitViewResizeSnapshot, SplitViewSashLayout,
};
pub use paint::{Border, BoxShadow, Color, PaintRect};
pub use renderer::{UiRenderError, UiRenderer, UiViewport};
pub use scene::{FontFamily, FontStyle, FontWeight, TextBlock, TextSpan, TextStyle, UiScene};
pub use text_input::{
    CaretBlinkAdvance, CaretBlinkController, CaretVisibility, TextInput, TextInputCommand,
    TextInputCompositionCursor, TextInputCompositionEvent, TextInputLayout, TextInputLayoutEngine,
    TextInputLayoutStyle, TextInputSelectionMode,
};
pub use text_layout::{TextLayout, TextLayoutEngine, TextLayoutWidth};
