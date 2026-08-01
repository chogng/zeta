//! Native UI scene and rendering infrastructure.

mod components;
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

pub use components::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarSeparatorStyle,
    ActionBarStyle, Button, ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle,
    Component, ComponentInspection, ContextMenu, ContextMenuItem, ContextMenuSelection,
    ContextMenuStyle, ContextView, ContextViewAnchorAlignment, ContextViewAnchorAxis,
    ContextViewAnchorPosition, ContextViewLayout, ContextViewPlacement, ContextViewStyle, Dropdown,
    DropdownItem, DropdownScrollConfiguration, DropdownSelection, DropdownStyle, IconLabel,
    IconLabelStyle, InputBox, InputBoxState, InputBoxStateColors, InputBoxStyle, Keycap,
    KeycapSequence, KeycapStyle, ListContentPadding, ListItemLayout, ListView, Sash,
    SashOrientation, SashState, SashStyle, ScrollAxis, ScrollCommand, ScrollDelta, ScrollMetrics,
    ScrollState, ScrollView, ScrollViewStyle, ScrollViewport, ScrollbarController, ScrollbarDrag,
    ScrollbarHit, ScrollbarLayout, ScrollbarPart, ScrollbarPointerPresence, ScrollbarPresentation,
    ScrollbarState, ScrollbarStyle, ScrollbarVisibility, SearchBox, SearchBoxStyle, Tab,
    TabBackgrounds, TabList, TabListOrientation, TabListStyle, TabSelection, TabState, TabStyle,
    TreeItem, TreeItemExpansion, TreeItemLayout, TreeView, TreeViewStyle, VirtualListLayout,
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
