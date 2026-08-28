//! App Server runtime, state, presentation, and interaction for Session.
//!
//! The runtime owns Session/Thread requests, subscriptions, and reconnects. The Pane owns the
//! backend-assembled transcript and one ChatWidget composed from the timeline and ChatInput
//! surfaces. Tabs, product effects, and other App Server capabilities remain outside this crate.

mod chat_input;
mod chat_input_editor;
mod chat_input_interaction;
mod chat_input_interaction_pane;
mod chat_input_pane;
mod chat_input_toolbar;
mod chat_widget;
pub mod interaction;
mod pane;
mod pane_context;
mod pane_state;
mod pane_style;
mod runtime;
mod runtime_contract;
mod runtime_worker;
mod session_canvas;
mod thread_timeline;
mod timeline_scroll;
mod transcript_state;

pub(crate) use chat_input::ChatInput;
pub use chat_input::ComposerRoute;
pub use chat_input::ComposerSubmission;
pub(crate) use chat_input_editor::ChatInputEditor;
pub(crate) use chat_input_editor::ChatInputFocus;
pub use chat_input_interaction::ChatInputInteractionItem;
pub(crate) use chat_input_interaction::ChatInputInteractionState;
pub use chat_input_interaction::ChatInputInteractionView;
pub use chat_input_interaction::ComposerInteractionActivation;
pub use chat_input_interaction::ComposerModelOption;
pub use chat_input_interaction::SelectionDirection;
pub(crate) use chat_input_interaction_pane::ChatInputInteractionPaneState;
pub use chat_input_pane::ComposerPanelLayout;
pub use chat_input_pane::INTERACTION_ROW_HEIGHT;
pub use chat_input_pane::interaction_content_size;
pub use chat_input_pane::interaction_list_bounds;
pub use chat_input_pane::interaction_preferred_height;
pub use chat_input_pane::interaction_selection_scroll_command;
pub use pane::{SessionPaneLayout, SessionPaneView, draw_session_pane};
pub use pane_context::SessionPaneContext;
pub use pane_state::SessionPaneState;
pub use pane_style::SessionPaneStyle;
pub use runtime::CommandResult;
pub use runtime::SESSION_UNAVAILABLE_COMMAND_ERROR;
pub use runtime::SessionRuntime;
pub use runtime::SessionRuntimeEvent;
pub(crate) use runtime::SessionRuntimeEventSink;
pub use runtime::SessionRuntimeTarget;
pub use runtime::WorkspaceSwitchResult;
pub(crate) use runtime_contract::RECONNECT_WINDOW;
pub(crate) use runtime_contract::SessionRuntimeCommand;
pub(crate) use runtime_contract::reconnect_delay_within_window;
pub(crate) use runtime_contract::reject_disconnected_command;
pub use session_canvas::{SessionCanvasLayout, SessionHeader, SessionHeaderStyle};
pub(crate) use thread_timeline::{ThreadTimeline, ThreadTimelineStyle, line_capacity, line_count};
pub use timeline_scroll::{ThreadTimelineScroll, TimelineScrollDelta};
pub(crate) use transcript_state::TranscriptState;
