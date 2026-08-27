//! Session state, lifecycle-facing models, and presentation.
//!
//! The crate accepts typed Thread snapshots, incremental updates, and host-resolved
//! style values, then exposes feature state and UI components. Window, transport,
//! and other product-host effects remain outside this crate.

pub mod interaction;
mod session_canvas;
mod session_context_menu;
mod session_search;
mod thread_state;
mod thread_timeline;
mod timeline_scroll;
mod workbench_input;

pub use session_canvas::{SessionCanvasLayout, SessionHeader, SessionHeaderStyle};
pub use session_context_menu::{
    SessionContextMenu, SessionContextMenuState, SessionContextMenuStyle,
    update_session_context_menu_pointer,
};
pub use session_search::SessionSearchState;
pub use thread_state::{ThreadState, ThreadUpdateResult};
pub use thread_timeline::{ThreadTimeline, ThreadTimelineStyle, line_capacity, line_count};
pub use timeline_scroll::{ThreadTimelineScroll, TimelineScrollDelta};
pub use workbench_input::session_tab_input;
