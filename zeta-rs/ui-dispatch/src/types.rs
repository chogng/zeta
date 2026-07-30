/// Stable identity for one component instance across presentation rebuilds.
///
/// Component hosts allocate the scope and local value. Dynamic consumers such as
/// file trees and editors should retain the same identity while the represented
/// item remains mounted.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(u64);

impl ElementId {
    pub const fn scoped(scope: u32, local: u32) -> Self {
        Self(((scope as u64) << 32) | local as u64)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CursorFeedback {
    #[default]
    Default,
    Text,
    Pointer,
    ResizeHorizontal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FocusBehavior {
    #[default]
    None,
    TabStop,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NavigationGroupId(ElementId);

impl NavigationGroupId {
    pub const fn new(element: ElementId) -> Self {
        Self(element)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NodeAction {
    #[default]
    None,
    Activate,
    StartWindowDrag,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiIntent {
    Activate(ElementId),
    StartWindowDrag(ElementId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityRole {
    Window,
    Group,
    Separator,
    Toolbar,
    Button,
    TextInput,
    Terminal,
    TabList,
    Tab,
    ScrollBar,
    Menu,
    MenuItem,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccessibilitySelection {
    #[default]
    NotApplicable,
    Selected,
    Unselected,
}
