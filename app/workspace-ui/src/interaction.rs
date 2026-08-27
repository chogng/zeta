//! Stable interaction identities owned by the Workspace UI feature.
//!
//! The shell can re-export these identities for event routing, while the feature
//! keeps the identity values next to the components that consume them.

use zui::ui::ElementId;

pub const WORKSPACE_PANE: ElementId = ElementId::scoped(1, 23);
pub const AGENT_EXPLORER_PANE: ElementId = ElementId::scoped(1, 28);
pub const AGENT_EDITOR_PANE: ElementId = ElementId::scoped(1, 29);
pub const MULTI_DIFF_EDITOR: ElementId = ElementId::scoped(1, 30);
pub const MULTI_DIFF_SCROLLBAR: ElementId = ElementId::scoped(1, 31);
pub const WORKSPACE_PANE_NAVIGATION: ElementId = ElementId::scoped(1, 32);
pub const AGENT_CHANGES: ElementId = ElementId::scoped(1, 33);
pub const AGENT_FILES: ElementId = ElementId::scoped(1, 34);
pub const WORKSPACE_PANE_TOOLBAR: ElementId = ElementId::scoped(1, 35);
pub const AGENT_FILES_ACTION_BAR: ElementId = ElementId::scoped(1, 36);
pub const AGENT_FILES_REFRESH: ElementId = ElementId::scoped(1, 37);
pub const AGENT_FILES_SEARCH: ElementId = ElementId::scoped(1, 38);
pub const AGENT_FILE_SEARCH_INPUT: ElementId = ElementId::scoped(1, 39);
pub const AGENT_FILES_TOOLBAR: ElementId = ElementId::scoped(1, 52);
