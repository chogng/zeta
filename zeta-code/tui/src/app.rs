mod active_overlay;
mod composer_mode;
mod bootstrap;
mod command;
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
mod state;
mod status_notice;
mod transcript_batch;

pub(crate) use crate::features::sessions::ActiveConversation;
pub(crate) use bootstrap::chat_input_catalog_snapshot;
pub(crate) use bootstrap::slash_command_registry;
pub(crate) use command::AppCommand;
pub(crate) use event::AppEvent;
pub(crate) use event_loop::run;
#[cfg(test)]
pub(crate) use request_completion::apply_active_turn_snapshot;
pub(crate) use state::App;
pub(crate) use state::Status;
