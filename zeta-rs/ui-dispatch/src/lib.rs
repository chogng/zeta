mod dispatch;
mod frame;
mod types;

pub use dispatch::{DispatchInvalidation, DispatchOutcome, FocusDirection, UiDispatch};
pub use frame::{AccessibilityNode, InteractionFrame, UiNode};
pub use types::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    NavigationAxis, NavigationGroupId, NodeAction, UiIntent,
};

#[cfg(test)]
#[path = "ui_dispatch_tests.rs"]
mod tests;
