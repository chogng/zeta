//! Backend-neutral native UI framework.
//!
//! `zui` owns declarative element layout, immutable scene composition, inspection metadata,
//! geometry, paint primitives, text layout, and backend-neutral interaction semantics. It does
//! not own reusable product components, platform event adapters, product commands, or a graphics
//! backend.

mod foundation;
mod layout;
mod presentation;
#[doc(hidden)]
pub mod renderer_support;
mod runtime;
mod text;

pub use foundation::Color;
pub use foundation::CornerRadii;
pub use foundation::Edges;
pub use foundation::Point;
pub use foundation::Rect;
pub use foundation::Size;
pub use layout::GridLayout;
pub use layout::GridLeafLayout;
pub use layout::GridNode;
pub use layout::GridPane;
pub use layout::GridSashLayout;
pub use layout::GridSplitLayout;
pub use layout::SplitViewLayout;
pub use layout::SplitViewLayoutPriority;
pub use layout::SplitViewOrientation;
pub use layout::SplitViewPane;
pub use layout::SplitViewResize;
pub use layout::SplitViewResizeSnapshot;
pub use layout::SplitViewSashLayout;
pub use presentation::Border;
pub use presentation::BoxShadow;
pub use presentation::Component;
pub use presentation::ComponentElement;
pub use presentation::ComputedElement;
pub use presentation::Element;
pub use presentation::ElementDirection;
pub use presentation::ElementLength;
pub use presentation::ElementStyle;
pub use presentation::ImageData;
pub use presentation::ImageDataError;
pub use presentation::ImageId;
pub use presentation::InspectionFrame;
pub use presentation::InspectionNode;
pub use presentation::InspectionNodeId;
pub use presentation::PaintIcon;
pub use presentation::PaintImage;
pub use presentation::PaintRect;
pub use presentation::SceneBatch;
pub use presentation::SceneCheckpoint;
pub use presentation::TextBlock;
pub use presentation::TextBlockWrap;
pub use presentation::UiScene;
pub use runtime::AccessibilityExpansion;
pub use runtime::AccessibilityNode;
pub use runtime::AccessibilityRole;
pub use runtime::AccessibilitySelection;
pub use runtime::CursorFeedback;
pub use runtime::DispatchInvalidation;
pub use runtime::DispatchOutcome;
pub use runtime::ElementId;
pub use runtime::FocusBehavior;
pub use runtime::FocusDirection;
pub use runtime::FrameInvalidation;
pub use runtime::FrameSchedule;
pub use runtime::FrameScheduler;
pub use runtime::InteractionFrame;
pub use runtime::InteractionFrameCheckpoint;
pub use runtime::NavigationAxis;
pub use runtime::NavigationGroupId;
pub use runtime::NodeAction;
pub use runtime::UiDispatch;
pub use runtime::UiIntent;
pub use runtime::UiNode;
pub use text::CaretBlinkAdvance;
pub use text::CaretBlinkController;
pub use text::CaretVisibility;
pub use text::FontCatalog;
pub use text::FontCatalogError;
pub use text::FontFamily;
pub use text::FontStyle;
pub use text::FontWeight;
pub use text::TextInput;
pub use text::TextInputCommand;
pub use text::TextInputCompositionCursor;
pub use text::TextInputCompositionEvent;
pub use text::TextInputLayout;
pub use text::TextInputLayoutEngine;
pub use text::TextInputLayoutStyle;
pub use text::TextInputSelectionMode;
pub use text::TextLayout;
pub use text::TextLayoutEngine;
pub use text::TextLayoutWidth;
pub use text::TextSpan;
pub use text::TextStyle;

#[cfg(test)]
#[path = "architecture_tests.rs"]
mod architecture_tests;
