mod bootstrap;
mod command;
mod composer_mode;
#[cfg(test)]
#[path = "app/conversation_flow_tests.rs"]
mod conversation_flow_tests;
mod dispatch;
mod escape;
mod event;
mod event_loop;
mod event_pump;
mod frame;
mod help;
mod recovery;
mod redraw;
mod request_completion;
mod screen_layout;
#[cfg(test)]
#[path = "app/session_manager_tests.rs"]
mod session_manager_tests;
mod state;
mod status_notice;
mod transcript_batch;

pub(crate) use crate::features::sessions::ActiveConversation;
pub(crate) use bootstrap::chat_input_catalog_snapshot;
pub(crate) use bootstrap::slash_command_registry;
pub(crate) use command::AppCommand;
#[cfg(test)]
pub(crate) use composer_mode::ComposerMode;
pub(crate) use event::AppEvent;
pub(crate) use event_loop::run;
#[cfg(test)]
pub(crate) use request_completion::apply_active_turn_snapshot;
pub(crate) use state::App;
pub(crate) use state::Status;
