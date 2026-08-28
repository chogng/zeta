//! Private structural state for Workbench tabs, panes, and mounted content identities.

mod panepart;
mod state;
mod tabpart;

pub(crate) use panepart::{
    Pane, PaneContainer, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind, PaneNode,
    PanePart, PaneSplitDirection, PaneSplitId,
};
pub(crate) use state::{ClosedTab, Workbench};
pub(crate) use tabpart::{
    TabGroup, TabGroupId, TabId, TabInput, TabInputChange, TabInputKey, TabInputMetadata, TabPart,
    TabStatus, TabStatusKind,
};
