//! Structural state for Workbench tabs, panes, and mounted content identities.

mod panepart;
mod tabpart;
mod workbench;

pub use panepart::{
    Pane, PaneContainer, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind, PaneNode,
    PanePart, PaneSplitDirection, PaneSplitId,
};
pub use tabpart::{
    TabGroup, TabGroupId, TabId, TabInput, TabInputChange, TabInputKey, TabInputMetadata, TabPart,
    TabStatus, TabStatusKind,
};
pub use workbench::{ClosedTab, Workbench};
