//! Session-facing product surfaces grouped under the Workbench application layer.
//!
//! These modules own tab projection, search, sidebar state, and session diagnostics. They remain
//! product modules; reusable UI primitives continue to live in `zeta-ui`/`zui`.

#[path = "session/session_canvas.rs"]
pub(crate) mod session_canvas;
#[path = "session/session_context_menu.rs"]
pub(crate) mod session_context_menu;
#[path = "session/session_search.rs"]
pub(crate) mod session_search;
#[path = "session/session_sidebar.rs"]
pub(crate) mod session_sidebar;
#[path = "session/session_sidebar_toolbar.rs"]
pub(crate) mod session_sidebar_toolbar;
#[path = "session/session_switch_trace.rs"]
pub(crate) mod session_switch_trace;
#[path = "session/session_tab_list.rs"]
pub(crate) mod session_tab_list;
