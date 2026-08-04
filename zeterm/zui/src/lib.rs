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

pub use foundation::AnimationBinding;
pub use foundation::AnimationEasing;
pub use foundation::AnimationKey;
pub use foundation::AnimationProperty;
pub use foundation::Color;
pub use foundation::CornerRadii;
pub use foundation::Edges;
pub use foundation::ElementId;
pub use foundation::FrameInvalidation;
pub use foundation::Icon;
pub use foundation::IconDefinition;
pub use foundation::IconId;
pub use foundation::IconRendering;
pub use foundation::InteractionSink;
pub use foundation::Point;
pub use foundation::Rect;
pub use foundation::ScalarAnimationSpec;
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
pub use presentation::ComponentContext;
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
pub use presentation::SceneFragmentError;
pub use presentation::TextBlock;
pub use presentation::TextBlockWrap;
pub use presentation::UiFrame;
pub use presentation::UiScene;
pub use runtime::AccessibilityExpansion;
pub use runtime::AccessibilityNode;
pub use runtime::AccessibilityRole;
pub use runtime::AccessibilitySelection;
pub use runtime::AnimationAdvance;
pub use runtime::AnimationAdvanceReport;
pub use runtime::AnimationRegistry;
pub use runtime::CursorFeedback;
pub use runtime::DispatchInvalidation;
pub use runtime::DispatchOutcome;
pub use runtime::FocusBehavior;
pub use runtime::FocusDirection;
pub use runtime::FrameDeadlineSet;
pub use runtime::FrameSchedule;
pub use runtime::FrameScheduler;
pub use runtime::InteractionFrame;
pub use runtime::InteractionFrameCheckpoint;
pub use runtime::NavigationAxis;
pub use runtime::NavigationGroupId;
pub use runtime::NodeAction;
pub use runtime::RetainedFragmentAdvanceReport;
pub use runtime::RetainedFragmentError;
pub use runtime::RetainedFragmentExit;
pub use runtime::RetainedFragmentMount;
pub use runtime::RetainedFragmentRegistry;
pub use runtime::RetainedFragmentState;
pub use runtime::RetainedRuntime;
pub use runtime::RetainedRuntimeAdvanceReport;
pub use runtime::ScalarAnimation;
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
