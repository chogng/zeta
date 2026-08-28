use super::ActiveConversation;
use super::App;
use super::AppEvent;
use super::dispatch::ProductCommandOutput;
use super::skill_slash_command_registry;
use crate::TuiExit;
use crate::TuiWorkspaceReconnect;
use crate::components::composer::ComposerSubmission;
use crate::components::composer::SlashCommandCatalog;
use crate::features::config;
use crate::features::interactions::InteractionResponse;
use crate::features::sessions::ConversationChange;
use crate::features::sessions::ConversationTranscript;
use crate::features::skills;
use crate::features::skills::SkillSelectionView;
use crate::features::thread::ActiveTurnUpdate;
use crate::features::thread::LatestThreadSnapshot;
use crate::features::thread::OlderThreadHistoryPage;
use crate::features::thread::ThreadRequestScope;
use crate::features::thread::ThreadSubscription;
use crate::features::thread::ThreadSwitch;
use crate::features::thread::evaluate_active_turn;
use crate::features::thread::interrupt_turn;
use crate::features::thread::read_thread_history;
use crate::features::thread::recover_active_turn;
use crate::features::thread::resolve_interaction;
use crate::features::thread::submit_prompt;
use std::collections::BTreeMap;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_protocol::SkillRef;
use zeta_protocol::Thread;
#[cfg(test)]
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

pub(super) enum RequestCompletion {
    WorkspaceReconnect(TuiWorkspaceReconnect),
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
    ApprovalModeChanged(Result<ActiveConversation, ClientError>),
    SkillsRefreshed(Result<SkillRequestCompletion, String>),
    InteractionResolved(Result<LatestThreadSnapshot, ClientError>),
    ThreadRefreshed(Result<LatestThreadSnapshot, ClientError>),
    ThreadHistoryPage(Result<OlderThreadHistoryPage, ClientError>),
    TurnInterrupted(Result<LatestThreadSnapshot, ClientError>),
    TurnStarted(Result<(TurnStartResult, LatestThreadSnapshot), ClientError>),
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

pub(super) struct SkillRequestCompletion {
    slash_commands: SlashCommandCatalog,
    skill_commands: BTreeMap<String, SkillRef>,
    view: SkillSelectionView,
}

pub(super) fn refresh_skills_and_registry(
    mut client: AppServerRequestHandle,
    server_slash_commands: Vec<SlashCommandDefinition>,
    session_id: zeta_protocol::SessionId,
) -> Result<SkillRequestCompletion, String> {
    let catalog = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Cached,
            session_id: Some(session_id),
        })
        .map_err(|error| error.to_string())?;
    let registry = skill_slash_command_registry(&server_slash_commands, &catalog)
        .map_err(|error| error.to_string())?;
    Ok(SkillRequestCompletion {
        slash_commands: registry.catalog,
        skill_commands: registry.skills,
        view: skills::skills_selection_view(&catalog),
    })
}

pub(super) fn resolve_interaction_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    response: InteractionResponse,
    history: ThreadSnapshotHistory,
) -> Result<LatestThreadSnapshot, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    resolve_interaction(
        &mut client,
        scope,
        response.turn_id,
        response.request_id,
        response.response,
    )?;
    read_thread_history(&mut client, &session_id, &thread_id, history)
}

pub(super) fn interrupt_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    turn_id: TurnId,
    history: ThreadSnapshotHistory,
) -> Result<LatestThreadSnapshot, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    interrupt_turn(&mut client, scope, &turn_id)?;
    read_thread_history(&mut client, &session_id, &thread_id, history)
}

pub(super) fn start_turn_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    submission: ComposerSubmission,
    history: ThreadSnapshotHistory,
) -> Result<(TurnStartResult, LatestThreadSnapshot), ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    let start = submit_prompt(&mut client, scope, submission)?;
    let snapshot = read_thread_history(&mut client, &session_id, &thread_id, history)?;
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
) -> Option<TuiExit> {
    match completion {
        RequestCompletion::WorkspaceReconnect(reconnect) => {
            return Some(TuiExit::WorkspaceReconnectRequested(reconnect));
        }
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
            if let Some(reconnect) = output.workspace_reconnect.take() {
                return Some(TuiExit::WorkspaceReconnectRequested(reconnect));
            }
            for event in output.events.drain(..) {
                app.update(event);
            }
            if output.exit_requested {
                return Some(TuiExit::UserRequested);
            }
            if let Some(change) = output.conversation_change.take() {
                let Some((subscription, switch)) = switched else {
                    app.update(AppEvent::FailureReported(
                        "conversation command completed without a subscription result".into(),
                    ));
                    return None;
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
        RequestCompletion::ApprovalModeChanged(Ok(next_conversation)) => {
            conversation.merge_session_from(next_conversation);
            app.set_next_approval_mode(conversation.next_approval_mode());
        }
        RequestCompletion::ApprovalModeChanged(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::PreferredModelUpdated {
            command,
            result: Ok(update),
        } => {
            app.update(AppEvent::PreferredModelReceived(update.preferred_model));
            app.update(AppEvent::CommandCompleted {
                command,
                result: update.notice,
            });
            app.update(AppEvent::SelectionViewClosed);
        }
        RequestCompletion::PreferredModelUpdated {
            result: Err(error), ..
        } => app.update(AppEvent::FailureReported(error)),
        RequestCompletion::SkillsRefreshed(Ok(refresh)) => {
            app.replace_slash_commands(refresh.slash_commands, refresh.skill_commands);
            if app.skills_view_is_active() {
                app.update(AppEvent::SkillsViewReplaced(refresh.view));
            }
        }
        RequestCompletion::SkillsRefreshed(Err(error)) => {
            if app.skills_view_is_active() {
                app.update(AppEvent::FailureReported(error));
            }
        }
        RequestCompletion::TurnStarted(Ok((start, snapshot))) => {
            conversation.set_thread_sequence(snapshot.thread.sequence.max(start.sequence));
            thread_subscription.apply_latest_snapshot(&snapshot.thread, snapshot.boundary);
            if active_turn.is_none() {
                *active_turn = Some(start.turn_id);
            }
            apply_thread_snapshot(app, active_turn, snapshot.thread, snapshot.transcript);
        }
        RequestCompletion::TurnStarted(Err(error)) => {
            report_turn_start_failure(app, active_turn, error.to_string());
        }
        RequestCompletion::InteractionResolved(Ok(snapshot)) => {
            conversation.set_thread_sequence(snapshot.thread.sequence);
            thread_subscription.apply_latest_snapshot(&snapshot.thread, snapshot.boundary);
            app.update(AppEvent::SelectionViewClosed);
            apply_thread_snapshot(app, active_turn, snapshot.thread, snapshot.transcript);
        }
        RequestCompletion::InteractionResolved(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::ThreadRefreshed(Ok(snapshot)) => {
            if snapshot.thread.session_id != *conversation.session_id()
                || snapshot.thread.thread_id != *conversation.thread_id()
            {
                app.update(AppEvent::FailureReported(format!(
                    "session/thread/read returned snapshot for {}/{}; expected {}/{}",
                    snapshot.thread.session_id,
                    snapshot.thread.thread_id,
                    conversation.session_id(),
                    conversation.thread_id()
                )));
                return None;
            }
            conversation.set_thread_sequence(snapshot.thread.sequence);
            thread_subscription.apply_latest_snapshot(&snapshot.thread, snapshot.boundary);
            apply_thread_snapshot(app, active_turn, snapshot.thread, snapshot.transcript);
        }
        RequestCompletion::ThreadHistoryPage(Ok(page)) => {
            if page.thread.session_id != *conversation.session_id()
                || page.thread.thread_id != *conversation.thread_id()
            {
                app.update(AppEvent::FailureReported(format!(
                    "session/thread/read returned history for {}/{}; expected {}/{}",
                    page.thread.session_id,
                    page.thread.thread_id,
                    conversation.session_id(),
                    conversation.thread_id()
                )));
                return None;
            }
            thread_subscription.apply_history_page(&page.thread, page.boundary);
            app.update(AppEvent::ThreadTranscriptHistoryPageReceived(
                page.transcript,
            ));
        }
        RequestCompletion::ThreadHistoryPage(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::ThreadRefreshed(Err(error)) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        RequestCompletion::TurnInterrupted(Ok(snapshot)) => {
            conversation.set_thread_sequence(snapshot.thread.sequence);
            thread_subscription.apply_latest_snapshot(&snapshot.thread, snapshot.boundary);
            apply_thread_snapshot(app, active_turn, snapshot.thread, snapshot.transcript);
        }
        RequestCompletion::TurnInterrupted(Err(error)) => {
            app.update(AppEvent::InterruptFailed(error.to_string()));
        }
    }
    None
}

fn report_turn_start_failure(app: &mut App, active_turn: &Option<TurnId>, error: String) {
    if active_turn.is_some() {
        app.update(AppEvent::HostOperationCompleted(Err(format!(
            "could not queue the follow-up: {error}"
        ))));
    } else {
        app.update(AppEvent::FailureReported(error));
    }
}

#[cfg(test)]
pub(crate) fn apply_active_turn_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    turns: &[Turn],
) {
    let update = evaluate_active_turn(active_turn, turns);
    apply_active_turn_update(app, update);
    if active_turn.is_none() {
        *active_turn = recover_active_turn(turns);
        if active_turn.is_some() {
            let next_update = evaluate_active_turn(active_turn, turns);
            apply_active_turn_update(app, next_update);
        }
    }
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

#[cfg(test)]
#[path = "request_completion_tests.rs"]
mod tests;

pub(super) fn apply_thread_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    snapshot: Thread,
    transcript: ThreadTranscriptSnapshot,
) {
    if active_turn.is_none() {
        *active_turn = recover_active_turn(&snapshot.turns);
    }
    let active_turn_update = evaluate_active_turn(active_turn, &snapshot.turns);
    let next_active_turn_update = if active_turn.is_none() {
        *active_turn = recover_active_turn(&snapshot.turns);
        if active_turn.is_some() {
            Some(evaluate_active_turn(active_turn, &snapshot.turns))
        } else {
            None
        }
    } else {
        None
    };
    app.update(AppEvent::ThreadTranscriptSnapshotReceived(transcript));
    apply_active_turn_update(app, active_turn_update);
    if let Some(next_active_turn_update) = next_active_turn_update {
        apply_active_turn_update(app, next_active_turn_update);
    }
    let current_approval_mode = active_turn.as_ref().and_then(|turn_id| {
        snapshot
            .turns
            .iter()
            .find(|turn| turn.turn_id == *turn_id)
            .map(|turn| turn.approval_mode)
    });
    app.set_current_approval_mode(current_approval_mode);
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
    app.set_next_approval_mode(conversation.next_approval_mode());
    if matches!(presentation, ConversationCompletionPresentation::Command(_)) {
        app.update(AppEvent::SelectionViewClosed);
    }
    if matches!(change.transcript, ConversationTranscript::Clear) {
        app.update(AppEvent::TranscriptCleared);
    }
    *active_turn = None;
    match switch {
        ThreadSwitch::Complete {
            snapshot,
            transcript,
        } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot, transcript);
        }
        ThreadSwitch::StaleSubscription {
            snapshot,
            transcript,
            error,
        } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, active_turn, snapshot, transcript);
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
