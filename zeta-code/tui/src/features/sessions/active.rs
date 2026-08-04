use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_app_server_protocol::protocol::session::SessionReadParams;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_app_server_protocol::protocol::session::SessionRequestResult;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadResult;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// Mutable product Session/Thread selection used by one TUI conversation.
pub(crate) struct ActiveConversation {
    session: Session,
    thread_id: ThreadId,
    thread_sequence: u64,
}

pub(crate) enum NewConversationKind {
    Clear,
    New,
}

pub(crate) struct ConversationChange {
    pub(crate) snapshot: Thread,
    pub(crate) notice: String,
    pub(crate) transcript: ConversationTranscript,
}

pub(crate) enum ConversationTranscript {
    Clear,
    Replace,
}

pub(crate) enum ResumeOutcome {
    Listed(String),
    Changed(ConversationChange),
}

impl ActiveConversation {
    pub(crate) fn start<T>(
        client: &mut AppServerClient<T>,
        title: String,
    ) -> Result<Self, ClientError>
    where
        T: JsonRpcTransport,
    {
        create_conversation(client, title).map(|(conversation, _)| conversation)
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session.session_id
    }

    pub(crate) fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }

    pub(crate) fn thread_sequence(&self) -> u64 {
        self.thread_sequence
    }

    pub(crate) fn set_thread_sequence(&mut self, sequence: u64) {
        self.thread_sequence = sequence;
    }

    pub(crate) fn replace_with_new<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        kind: NewConversationKind,
        arguments: &str,
    ) -> Result<ConversationChange, SessionsError>
    where
        T: JsonRpcTransport,
    {
        let title = if arguments.is_empty() {
            match kind {
                NewConversationKind::Clear => "Cleared conversation",
                NewConversationKind::New => "TUI conversation",
            }
            .to_owned()
        } else {
            arguments.to_owned()
        };
        let (conversation, snapshot) = create_conversation(client, title)?;
        *self = conversation;
        Ok(ConversationChange {
            snapshot,
            notice: format!(
                "Started session {} on thread {}.",
                self.session.session_id, self.thread_id
            ),
            transcript: ConversationTranscript::Clear,
        })
    }

    pub(crate) fn fork_active_thread<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        arguments: &str,
    ) -> Result<ConversationChange, SessionsError>
    where
        T: JsonRpcTransport,
    {
        let title = if arguments.is_empty() {
            format!("Fork of {}", self.session.title)
        } else {
            arguments.to_owned()
        };
        let result = client
            .request_session(SessionRequestParams {
                command_id: new_command_id("fork"),
                session_id: self.session.session_id.clone(),
                expected_sequence: self.session.sequence,
                request: SessionRequest::ForkThread {
                    parent_thread_id: self.thread_id.clone(),
                    title,
                },
            })
            .and_then(expect_thread_result)?;
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: result.session.session_id.clone(),
                thread_id: result.thread_id.clone(),
            })?
            .thread;
        self.session = result.session;
        self.thread_id = result.thread_id;
        self.thread_sequence = snapshot.sequence;
        Ok(ConversationChange {
            snapshot,
            notice: format!("Forked to thread {}.", self.thread_id),
            transcript: ConversationTranscript::Replace,
        })
    }

    pub(crate) fn rewind_active_thread<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        before_turn_id: TurnId,
        checkpoint_label: &str,
    ) -> Result<ConversationChange, SessionsError>
    where
        T: JsonRpcTransport,
    {
        let title = format!("Rewind of {}", self.session.title);
        let result = client
            .request_session(SessionRequestParams {
                command_id: new_command_id("rewind"),
                session_id: self.session.session_id.clone(),
                expected_sequence: self.session.sequence,
                request: SessionRequest::RewindThread {
                    parent_thread_id: self.thread_id.clone(),
                    before_turn_id,
                    title,
                },
            })
            .and_then(expect_thread_result)?;
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: result.session.session_id.clone(),
                thread_id: result.thread_id.clone(),
            })?
            .thread;
        self.session = result.session;
        self.thread_id = result.thread_id;
        self.thread_sequence = snapshot.sequence;
        Ok(ConversationChange {
            snapshot,
            notice: format!("Rewound before: {checkpoint_label}"),
            transcript: ConversationTranscript::Replace,
        })
    }

    pub(crate) fn resume_session<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        arguments: &str,
    ) -> Result<ResumeOutcome, SessionsError>
    where
        T: JsonRpcTransport,
    {
        if arguments.is_empty() {
            let sessions = client.list_sessions()?.sessions;
            let text = if sessions.is_empty() {
                "No saved sessions.".into()
            } else {
                let lines = sessions
                    .into_iter()
                    .map(|session| {
                        format!(
                            "{}  {}  {:?}",
                            session.session_id, session.title, session.status
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Saved sessions:\n{lines}\nUse /resume <session-id>.")
            };
            return Ok(ResumeOutcome::Listed(text));
        }

        let session_id = SessionId::new(arguments)
            .map_err(|error| SessionsError(format!("invalid session ID '{arguments}': {error}")))?;
        let session = client
            .read_session(SessionReadParams { session_id })?
            .session;
        let thread_id = session
            .threads
            .iter()
            .rev()
            .find(|thread| thread.status == SessionThreadStatus::Active)
            .map(|thread| thread.thread_id.clone())
            .ok_or_else(|| {
                SessionsError(format!(
                    "session {} has no active thread",
                    session.session_id
                ))
            })?;
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: session.session_id.clone(),
                thread_id: thread_id.clone(),
            })?
            .thread;
        self.session = session;
        self.thread_id = thread_id;
        self.thread_sequence = snapshot.sequence;
        Ok(ResumeOutcome::Changed(ConversationChange {
            snapshot,
            notice: format!(
                "Resumed session {} on thread {}.",
                self.session.session_id, self.thread_id
            ),
            transcript: ConversationTranscript::Replace,
        }))
    }
}

fn create_conversation<T>(
    client: &mut AppServerClient<T>,
    title: String,
) -> Result<(ActiveConversation, Thread), ClientError>
where
    T: JsonRpcTransport,
{
    let session = client.create_session(SessionCreateParams {
        command_id: new_command_id("session"),
        title: title.clone(),
    })?;
    let thread = client
        .request_session(SessionRequestParams {
            command_id: new_command_id("thread"),
            session_id: session.session.session_id.clone(),
            expected_sequence: session.session.sequence,
            request: SessionRequest::CreateThread { title },
        })
        .and_then(expect_thread_result)?;
    let snapshot = client
        .read_session_thread(SessionThreadReadParams {
            session_id: thread.session.session_id.clone(),
            thread_id: thread.thread_id.clone(),
        })?
        .thread;
    Ok((
        ActiveConversation {
            session: thread.session,
            thread_id: thread.thread_id,
            thread_sequence: snapshot.sequence,
        },
        snapshot,
    ))
}

fn expect_thread_result(result: SessionRequestResult) -> Result<SessionThreadResult, ClientError> {
    match result {
        SessionRequestResult::Thread(result) => Ok(result),
        other => Err(ClientError::Protocol(format!(
            "session request returned {other:?} for a Thread operation"
        ))),
    }
}

#[derive(Debug)]
pub(crate) struct SessionsError(String);

impl fmt::Display for SessionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<ClientError> for SessionsError {
    fn from(error: ClientError) -> Self {
        Self(error.to_string())
    }
}
