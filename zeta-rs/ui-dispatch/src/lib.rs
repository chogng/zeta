mod dispatch;
mod frame;
mod types;

pub use dispatch::{DispatchInvalidation, DispatchOutcome, FocusDirection, UiDispatch};
pub use frame::{AccessibilityNode, InteractionFrame, InteractionFrameCheckpoint, UiNode};
pub use types::{
    AccessibilityExpansion, AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId,
    FocusBehavior, NavigationAxis, NavigationGroupId, NodeAction, UiIntent,
};

#[cfg(test)]
#[path = "ui_dispatch_tests.rs"]
mod tests;
