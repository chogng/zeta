mod active;
mod completion;
#[cfg(test)]
mod completion_tests;
mod manager;
mod picker;
mod state;

pub(crate) use active::ActiveConversation;
pub(crate) use active::ConversationChange;
pub(crate) use active::ConversationTranscript;
pub(crate) use active::ResumeOutcome;
pub(crate) use completion::CommandRequest;
pub(crate) use completion::ConversationCompletion;
pub(crate) use completion::ManagerSessionCompletion;
pub(crate) use completion::SessionCompletion;
pub(crate) use completion::finish_conversation_request;
pub(crate) use completion::prepare_command;
pub(crate) use manager::SessionManagerPointerTarget;
pub(crate) use manager::SessionManagerView;
pub(crate) use manager::draw_manager;
pub(crate) use manager::pointer_target_at;
pub(crate) use picker::SessionChoices;
pub(crate) use picker::SessionSelectionAction;
pub(crate) use picker::session_choices;
pub(crate) use state::SessionsState;
pub(crate) use state::TerminalScreen;

use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_protocol::Session;
use zeta_protocol::SessionId;

/// A completed session operation delivered to the TUI state owner.
pub(crate) enum Event {
    PickerOpened(SessionChoices),
    CatalogReceived(Vec<Session>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Resume {
        session_id: String,
        preferred_thread_id: Option<zeta_protocol::ThreadId>,
    },
    Archive {
        session_ids: Vec<SessionId>,
    },
    CreateAndEnter {
        submission: crate::thread::composer::ChatSubmission,
    },
    SwitchThread {
        thread_id: zeta_protocol::ThreadId,
    },
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    active_session_id: &str,
) -> Result<SessionChoices, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_sessions()
        .map(|result| session_choices(&result.sessions, active_session_id))
}

pub(crate) fn load_catalog<T>(
    client: &mut AppServerClient<T>,
) -> Result<Vec<zeta_protocol::Session>, ClientError>
where
    T: JsonRpcTransport,
{
    client.list_sessions().map(|result| result.sessions)
}

pub(crate) fn archive<T>(
    client: &mut AppServerClient<T>,
    session_ids: Vec<SessionId>,
) -> Result<Vec<Session>, ClientError>
where
    T: JsonRpcTransport,
{
    for session_id in session_ids {
        client
            .request_session(SessionRequestParams {
                command_id: crate::client::new_command_id("archive"),
                session_id,
                request: SessionRequest::Archive,
            })
            .and_then(active::expect_session_result)?;
    }
    load_catalog(client)
}

pub(super) fn branch_count_label(session: &Session) -> String {
    let count = session.threads.len();
    format!("{count} {}", if count == 1 { "branch" } else { "branches" })
}

pub(super) fn session_size_label(session: &Session) -> String {
    let mut tokens = 0u64;
    let mut complete = true;
    for thread in &session.threads {
        tokens = tokens
            .saturating_add(thread.usage.input_tokens.reported)
            .saturating_add(thread.usage.output_tokens.reported);
        complete &= thread.usage.input_tokens.complete && thread.usage.output_tokens.complete;
    }
    let prefix = if complete { "" } else { "≥" };
    format!("{prefix}{} tokens", compact_count(tokens))
}

fn compact_count(count: u64) -> String {
    if count < 1_000 {
        return count.to_string();
    }
    if count < 1_000_000 {
        return format!("{:.1}K", count as f64 / 1_000.0);
    }
    format!("{:.1}M", count as f64 / 1_000_000.0)
}
