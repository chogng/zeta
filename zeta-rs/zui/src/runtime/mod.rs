//! Cross-frame coordination that remains independent from presentation composition.

mod frame_scheduler;
mod interaction;

pub use frame_scheduler::FrameInvalidation;
pub use frame_scheduler::FrameSchedule;
pub use frame_scheduler::FrameScheduler;
pub use interaction::AccessibilityExpansion;
pub use interaction::AccessibilityNode;
pub use interaction::AccessibilityRole;
pub use interaction::AccessibilitySelection;
pub use interaction::CursorFeedback;
pub use interaction::DispatchInvalidation;
pub use interaction::DispatchOutcome;
pub use interaction::ElementId;
pub use interaction::FocusBehavior;
pub use interaction::FocusDirection;
pub use interaction::InteractionFrame;
pub use interaction::InteractionFrameCheckpoint;
pub use interaction::NavigationAxis;
pub use interaction::NavigationGroupId;
pub use interaction::NodeAction;
pub use interaction::UiDispatch;
pub use interaction::UiIntent;
pub use interaction::UiNode;
