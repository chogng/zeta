//! Dependency-free value types shared by every framework layer.

mod animation;
mod color;
mod geometry;
mod identity;
mod interaction;
mod scheduling;

pub use animation::AnimationBinding;
pub use animation::AnimationEasing;
pub use animation::AnimationKey;
pub use animation::AnimationProperty;
pub use animation::ScalarAnimationSpec;
pub use color::Color;
pub use geometry::CornerRadii;
pub use geometry::Edges;
pub use geometry::Point;
pub use geometry::Rect;
pub use geometry::Size;
pub use identity::ElementId;
pub use interaction::AccessibilityExpansion;
pub use interaction::AccessibilityRole;
pub use interaction::AccessibilitySelection;
pub use interaction::CursorFeedback;
pub use interaction::DispatchInvalidation;
pub use interaction::FocusBehavior;
pub use interaction::InteractionSink;
pub use interaction::NavigationAxis;
pub use interaction::NavigationGroupId;
pub use interaction::NodeAction;
pub use interaction::UiIntent;
pub use interaction::UiNode;
pub use scheduling::FrameInvalidation;
