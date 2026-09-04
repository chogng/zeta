use super::ActiveConversation;
use super::Command;
use super::ConversationChange;
use super::ResumeOutcome;
use super::archive;
use crate::thread::ThreadRequestScope;
use crate::thread::ThreadSubscription;
use crate::thread::ThreadSwitch;
use crate::thread::TurnStartCompletion;
use crate::thread::composer::ChatSubmission;
use crate::thread::start_turn_and_read;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_protocol::ApprovalMode;
use zeta_protocol::Session;

pub(crate) struct ConversationCompletion {
    pub(crate) conversation: ActiveConversation,
    pub(crate) change: ConversationChange,
    pub(crate) subscription: ThreadSubscription,
    pub(crate) switch: ThreadSwitch,
}

pub(crate) struct ManagerSessionCompletion {
    pub(crate) conversation: ConversationCompletion,
    pub(crate) turn: TurnStartCompletion,
}

/// Result of one asynchronous Session or active-conversation operation.
pub(crate) enum SessionCompletion {
    Catalog(Result<Vec<Session>, String>),
    Changed {
        command: String,
        result: Result<ConversationCompletion, String>,
    },
    ThreadChanged(Result<ConversationCompletion, String>),
    ManagerCreated(Result<ManagerSessionCompletion, String>),
}

pub(crate) enum CommandRequest {
    Resume {
        session_id: String,
        preferred_thread_id: Option<zeta_protocol::ThreadId>,
    },
    Archive {
        session_ids: Vec<zeta_protocol::SessionId>,
    },
    CreateAndEnter {
        submission: ChatSubmission,
        approval_mode: ApprovalMode,
    },
    SwitchThread {
        thread_id: zeta_protocol::ThreadId,
    },
}

impl Command {
    pub(crate) fn command_line(&self) -> Option<String> {
        match self {
            Self::Resume { session_id, .. } => Some(format!("/resume {session_id}")),
            Self::Archive { .. } | Self::CreateAndEnter { .. } | Self::SwitchThread { .. } => None,
        }
    }
}

impl CommandRequest {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Resume { .. } => "zeta-tui-resume-session",
            Self::Archive { .. } => "zeta-tui-archive-sessions",
            Self::CreateAndEnter { .. } => "zeta-tui-create-manager-session",
            Self::SwitchThread { .. } => "zeta-tui-switch-thread",
        }
    }

    pub(crate) fn execute(
        self,
        mut client: AppServerRequestHandle,
        mut conversation: ActiveConversation,
        subscription: ThreadSubscription,
    ) -> SessionCompletion {
        match self {
            Self::Resume {
                session_id,
                preferred_thread_id,
            } => {
                let command = format!("/resume {session_id}");
                let result = match conversation.resume_session(
                    &mut client,
                    &session_id,
                    preferred_thread_id.as_ref(),
                ) {
                    Ok(ResumeOutcome::Changed(change)) => {
                        finish_conversation_request(&mut client, conversation, subscription, change)
                    }
                    Ok(ResumeOutcome::Listed(_)) => {
                        Err("resume selection did not identify a session".into())
                    }
                    Err(error) => Err(error.to_string()),
                };
                SessionCompletion::Changed { command, result }
            }
            Self::Archive { session_ids } => SessionCompletion::Catalog(
                archive(&mut client, session_ids).map_err(|error| error.to_string()),
            ),
            Self::CreateAndEnter {
                submission,
                approval_mode,
            } => SessionCompletion::ManagerCreated(create_manager_session_and_start(
                client,
                conversation,
                subscription,
                submission,
                approval_mode,
            )),
            Self::SwitchThread { thread_id } => {
                let result = conversation
                    .select_thread(&mut client, thread_id)
                    .map_err(|error| error.to_string())
                    .and_then(|change| {
                        finish_conversation_request(&mut client, conversation, subscription, change)
                    });
                SessionCompletion::ThreadChanged(result)
            }
        }
    }
}

pub(crate) fn prepare_command(approval_mode: ApprovalMode, command: Command) -> CommandRequest {
    match command {
        Command::Resume {
            session_id,
            preferred_thread_id,
        } => CommandRequest::Resume {
            session_id,
            preferred_thread_id,
        },
        Command::Archive { session_ids } => CommandRequest::Archive { session_ids },
        Command::CreateAndEnter { submission } => CommandRequest::CreateAndEnter {
            submission,
            approval_mode,
        },
        Command::SwitchThread { thread_id } => CommandRequest::SwitchThread { thread_id },
    }
}

pub(crate) fn finish_conversation_request(
    client: &mut AppServerRequestHandle,
    conversation: ActiveConversation,
    mut subscription: ThreadSubscription,
    change: ConversationChange,
) -> Result<ConversationCompletion, String> {
    let switch = subscription
        .switch(client, conversation.session_id(), conversation.thread_id())
        .map_err(subscription_error)?;
    Ok(ConversationCompletion {
        conversation,
        change,
        subscription,
        switch,
    })
}

pub(crate) fn create_manager_session_and_start(
    mut client: AppServerRequestHandle,
    mut conversation: ActiveConversation,
    subscription: ThreadSubscription,
    submission: ChatSubmission,
    approval_mode: ApprovalMode,
) -> Result<ManagerSessionCompletion, String> {
    let title = submission.display_text.clone();
    let change = conversation
        .replace_with_new(&mut client, &title)
        .map_err(|error| error.to_string())?;
    let conversation =
        finish_conversation_request(&mut client, conversation, subscription, change)?;
    let scope = ThreadRequestScope::new(
        conversation.conversation.session_id(),
        conversation.conversation.thread_id(),
        conversation.conversation.thread_sequence(),
    );
    let turn = start_turn_and_read(
        client,
        scope,
        submission,
        approval_mode,
        conversation.subscription.history(),
    );
    Ok(ManagerSessionCompletion { conversation, turn })
}

pub(crate) fn subscription_error(error: ClientError) -> String {
    format!("the command changed the conversation, but the TUI could not subscribe to it: {error}")
}
