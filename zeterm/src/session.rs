//! Session-facing feature surfaces projected into the Workbench application layer.
//!
//! These modules own Session canvas, search, context actions, and switching diagnostics. Workbench
//! Tab projection lives in `workbench_host`, while reusable UI primitives continue to live in
//! `zeta-ui`/`zui`.

#[path = "session/session_canvas.rs"]
pub(crate) mod session_canvas;
#[path = "session/session_context_menu.rs"]
pub(crate) mod session_context_menu;
#[path = "session/session_search.rs"]
pub(crate) mod session_search;
#[path = "session/session_switch_trace.rs"]
pub(crate) mod session_switch_trace;
