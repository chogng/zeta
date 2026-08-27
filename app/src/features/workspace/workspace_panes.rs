//! Compatibility exports for the Workspace UI feature crate.
//!
//! Workspace pane state, layout, rendering, and interaction live in
//! `zeta-workspace-ui`. The app host keeps this module so existing product wiring can
//! migrate without changing its import paths while it continues to own App Server
//! snapshots and side effects.

pub(crate) use zeta_workspace_ui::*;
