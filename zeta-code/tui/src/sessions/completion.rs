use super::ActiveConversation;
use super::ConversationChange;
use crate::thread::ThreadRequestScope;
use crate::thread::ThreadSubscription;
use crate::thread::ThreadSwitch;
use crate::thread::TurnStartCompletion;
use crate::thread::composer::ChatSubmission;
use crate::thread::start_turn_and_read;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_protocol::ApprovalMode;

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
    Changed {
        command: String,
        result: Result<ConversationCompletion, String>,
    },
    ThreadChanged(Result<ConversationCompletion, String>),
    ManagerCreated(Result<ManagerSessionCompletion, String>),
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
