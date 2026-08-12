use super::ActiveConversation;
use super::App;
use super::AppEvent;
use super::dispatch::ProductCommandOutput;
use crate::components::composer::ComposerSubmission;
use crate::features::config;
use crate::features::interactions::InteractionResponse;
use crate::features::sessions::ConversationChange;
use crate::features::sessions::ConversationTranscript;
use crate::features::thread::ActiveTurnUpdate;
use crate::features::thread::ThreadRequestScope;
use crate::features::thread::ThreadSubscription;
use crate::features::thread::ThreadSwitch;
use crate::features::thread::evaluate_active_turn;
use crate::features::thread::interrupt_turn;
use crate::features::thread::read_thread;
use crate::features::thread::recover_active_turn;
use crate::features::thread::resolve_interaction;
use crate::features::thread::submit_prompt;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_protocol::Thread;
#[cfg(test)]
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

pub(super) enum RequestCompletion {
    ConversationChanged {
        command: String,
        result: Result<ConversationRequestCompletion, String>,
    },
    ProductCommand(Result<ProductCommandCompletion, String>),
    Presentation(Result<AppEvent, String>),
    PreferredModelUpdated {
        command: String,
        result: Result<config::PreferredModelUpdate, String>,
    },
    InteractionResolved(Result<Thread, ClientError>),
    ThreadRefreshed(Result<Thread, ClientError>),
    TurnInterrupted(Result<Thread, ClientError>),
    TurnStarted(Result<(TurnStartResult, Thread), ClientError>),
}

pub(super) struct ConversationRequestCompletion {
    conversation: ActiveConversation,
    change: ConversationChange,
    subscription: ThreadSubscription,
    switch: ThreadSwitch,
}

pub(super) struct ProductCommandCompletion {
    output: ProductCommandOutput,
    switched: Option<(ThreadSubscription, ThreadSwitch)>,
}

pub(super) fn resolve_interaction_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    response: InteractionResponse,
) -> Result<Thread, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    resolve_interaction(
        &mut client,
        scope,
        response.turn_id,
        response.request_id,
        response.response,
    )?;
    read_thread(&mut client, &session_id, &thread_id)
}

pub(super) fn interrupt_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    turn_id: TurnId,
) -> Result<Thread, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    interrupt_turn(&mut client, scope, &turn_id)?;
    read_thread(&mut client, &session_id, &thread_id)
}

pub(super) fn start_turn_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    submission: ComposerSubmission,
) -> Result<(TurnStartResult, Thread), ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    let start = submit_prompt(&mut client, scope, submission)?;
    let snapshot = read_thread(&mut client, &session_id, &thread_id)?;
    Ok((start, snapshot))
}

pub(super) fn finish_conversation_request(
    client: &mut AppServerRequestHandle,
    conversation: ActiveConversation,
    mut subscription: ThreadSubscription,
    change: ConversationChange,
) -> Result<ConversationRequestCompletion, String> {
    let switch = subscription
        .switch(client, conversation.session_id(), conversation.thread_id())
        .map_err(subscription_error)?;
    Ok(ConversationRequestCompletion {
        conversation,
        change,
        subscription,
        switch,
    })
}

pub(super) fn finish_product_command_request(
    client: &mut AppServerRequestHandle,
    subscription: ThreadSubscription,
    output: ProductCommandOutput,
) -> Result<ProductCommandCompletion, String> {
    let switched = if output.conversation_change.is_some() {
        let mut subscription = subscription;
        let switch = subscription
            .switch(
                client,
                output.conversation.session_id(),
                output.conversation.thread_id(),
            )
            .map_err(subscription_error)?;
        Some((subscription, switch))
    } else {
        None
    };
    Ok(ProductCommandCompletion { output, switched })
}

fn subscription_error(error: ClientError) -> String {
    format!("the command changed the conversation, but the TUI could not subscribe to it: {error}")
}

pub(super) fn apply_request_completion(
    completion: RequestCompletion,
    conversation: &mut ActiveConversation,
    active_turn: &mut Option<TurnId>,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) {
    match completion {
        RequestCompletion::ConversationChanged {
            command,
            result:
                Ok(ConversationRequestCompletion {
                    conversation: next_conversation,
                    change,
                    subscription,
                    switch,
                }),
        } => {
            *conversation = next_conversation;
            *thread_subscription = subscription;
            finish_conversation_change(
                conversation,
                active_turn,
                app,
                change,
                switch,
                ConversationCompletionPresentation::Command(command),
            );
        }
        RequestCompletion::ConversationChanged {
            result: Err(error), ..
        }
        | RequestCompletion::ProductCommand(Err(error))
        | RequestCompletion::Presentation(Err(error)) => {
            app.update(AppEvent::FailureReported(error));
        }
        RequestCompletion::ProductCommand(Ok(ProductCommandCompletion {
            mut output,
            switched,
        })) => {
            for event in output.events.drain(..) {
                app.update(event);
            }
            if let Some(change) = output.conversation_change.take() {
                let Some((subscription, switch)) = switched else {
                    app.update(AppEvent::FailureReported(
                        "conversation command completed without a subscription result".into(),
                    ));
                    return;
                };
                *conversation = output.conversation;
                *thread_subscription = subscription;
                finish_conversation_change(
                    conversation,
                    active_turn,
                    app,
                    change,
                    switch,
                    ConversationCompletionPresentation::Notice,
                );
            }
        }
        RequestCompletion::Presentation(Ok(event)) => app.update(event),
        RequestCompletion::PreferredModelUpdated {
            command,
            result: Ok(update),
        } => {
            app.update(AppEvent::ConfigSnapshotReceived(update.config));
            app.update(AppEvent::CommandCompleted {
                command,
                result: update.notice,
            });
            app.update(AppEvent::SelectionViewClosed);
        }
        RequestCompletion::PreferredModelUpdated {
            result: Err(error), ..
        } => app.update(AppEvent::FailureReported(error)),
        RequestCompletion::TurnStarted(Ok((start, snapshot))) => {
            conversation.set_thread_sequence(snapshot.sequence.max(start.sequence));
            thread_subscription.confirm_sequence(snapshot.sequence);
            *active_turn = Some(start.turn_id);
            apply_thread_snapshot(app, active_turn, snapshot);
        }
        RequestCompletion::TurnStarted(Err(error)) => {
            *active_turn = None;
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::InteractionResolved(Ok(snapshot)) => {
            conversation.set_thread_sequence(snapshot.sequence);
            thread_subscription.confirm_sequence(snapshot.sequence);
            app.update(AppEvent::SelectionViewClosed);
            apply_thread_snapshot(app, active_turn, snapshot);
        }
        RequestCompletion::InteractionResolved(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::ThreadRefreshed(Ok(snapshot)) => {
            if snapshot.session_id != *conversation.session_id()
                || snapshot.thread_id != *conversation.thread_id()
            {
                app.update(AppEvent::FailureReported(format!(
                    "session/thread/read returned snapshot for {}/{}; expected {}/{}",
                    snapshot.session_id,
                    snapshot.thread_id,
                    conversation.session_id(),
                    conversation.thread_id()
                )));
                return;
            }
            conversation.set_thread_sequence(snapshot.sequence);
            thread_subscription.confirm_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot);
        }
        RequestCompletion::ThreadRefreshed(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::TurnInterrupted(Ok(snapshot)) => {
            conversation.set_thread_sequence(snapshot.sequence);
            thread_subscription.confirm_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot);
        }
        RequestCompletion::TurnInterrupted(Err(error)) => {
            app.update(AppEvent::InterruptFailed(error.to_string()));
        }
    }
}

#[cfg(test)]
pub(crate) fn apply_active_turn_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    turns: &[Turn],
) {
    apply_active_turn_update(app, evaluate_active_turn(active_turn, turns));
}

fn apply_active_turn_update(app: &mut App, update: ActiveTurnUpdate) {
    match update {
        ActiveTurnUpdate::ActivityChanged(activity) => {
            app.update(AppEvent::TurnActivityChanged(activity));
        }
        ActiveTurnUpdate::Completed => app.update(AppEvent::TurnCompleted),
        ActiveTurnUpdate::Failed(error) => app.update(AppEvent::FailureReported(error)),
        ActiveTurnUpdate::Interrupted => app.update(AppEvent::TurnInterrupted),
        ActiveTurnUpdate::Unchanged => {}
    }
}

pub(super) fn apply_thread_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    snapshot: Thread,
) {
    if active_turn.is_none() {
        *active_turn = recover_active_turn(&snapshot.turns);
    }
    let active_turn_update = evaluate_active_turn(active_turn, &snapshot.turns);
    app.update(AppEvent::ThreadSnapshotReceived(snapshot));
    apply_active_turn_update(app, active_turn_update);
}

enum ConversationCompletionPresentation {
    Command(String),
    Notice,
}

fn finish_conversation_change(
    conversation: &mut ActiveConversation,
    active_turn: &mut Option<TurnId>,
    app: &mut App,
    change: ConversationChange,
    switch: ThreadSwitch,
    presentation: ConversationCompletionPresentation,
) {
    if matches!(presentation, ConversationCompletionPresentation::Command(_)) {
        app.update(AppEvent::SelectionViewClosed);
    }
    if matches!(change.transcript, ConversationTranscript::Clear) {
        app.update(AppEvent::TranscriptCleared);
    }
    *active_turn = None;
    match switch {
        ThreadSwitch::Complete { snapshot } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot);
        }
        ThreadSwitch::StaleSubscription { snapshot, error } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot);
            app.update(AppEvent::FailureReported(format!(
                "changed Thread, but could not unsubscribe the previous Thread: {error}"
            )));
        }
    }
    match presentation {
        ConversationCompletionPresentation::Command(command) => {
            app.update(AppEvent::CommandCompleted {
                command,
                result: change.notice,
            });
        }
        ConversationCompletionPresentation::Notice => {
            app.update(AppEvent::ProductNotice(change.notice));
        }
    }
}
