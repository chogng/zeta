use super::ActiveConversation;
use super::App;
use super::AppEvent;
use super::dispatch::ProductCommandOutput;
use crate::config;
use crate::keymap;
use crate::models;
use crate::render::RenderTheme;
use crate::sessions::ConversationChange;
use crate::sessions::ConversationCompletion;
use crate::sessions::ConversationTranscript;
use crate::sessions::ManagerSessionCompletion;
use crate::sessions::SessionCompletion;
use crate::skills::SkillRefreshCompletion;
use crate::status::StatusLineSettings;
use crate::thread::ActiveTurnUpdate;
use crate::thread::ThreadCompletion;
use crate::thread::ThreadRequestScope;
use crate::thread::ThreadSubscription;
use crate::thread::ThreadSwitch;
use crate::thread::TurnStartCompletion;
use crate::thread::queue::QueueId;
use std::time::Instant;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_protocol::Thread;
#[cfg(test)]
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

pub(super) enum Completion {
    ConfigRefreshed(Result<ConfigReadResult, String>),
    Sessions(SessionCompletion),
    ProductCommand(Result<ProductCommandCompletion, String>),
    Presentation(Result<AppEvent, String>),
    PreferredModelUpdated {
        command: String,
        result: Result<models::PreferredModelUpdate, String>,
    },
    Skills(Result<SkillRefreshCompletion, String>),
    ThemeUpdated {
        command: String,
        label: String,
        theme: RenderTheme,
        result: Result<(), String>,
    },
    Thread(ThreadCompletion),
}

impl Completion {
    fn thread_scope(&self) -> Option<&ThreadRequestScope> {
        match self {
            Self::Thread(completion) => Some(completion.scope()),
            _ => None,
        }
    }
}

pub(super) struct ProductCommandCompletion {
    output: ProductCommandOutput,
    switched: Option<(ThreadSubscription, ThreadSwitch)>,
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
    completion: Completion,
    conversation: &mut ActiveConversation,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) {
    if completion
        .thread_scope()
        .is_some_and(|scope| !scope.targets(conversation.session_id(), conversation.thread_id()))
    {
        return;
    }
    match completion {
        Completion::ConfigRefreshed(Ok(config)) => apply_tui_config(config, None, app),
        Completion::ConfigRefreshed(Err(error)) => {
            app.update(AppEvent::FailureReported(error));
        }
        Completion::Sessions(SessionCompletion::ManagerCreated(Ok(ManagerSessionCompletion {
            conversation:
                ConversationCompletion {
                    conversation: next_conversation,
                    change,
                    subscription,
                    switch,
                },
            turn,
        }))) => {
            *conversation = next_conversation;
            *thread_subscription = subscription;
            finish_conversation_change(
                conversation,
                app,
                change,
                switch,
                ConversationCompletionPresentation::Silent,
            );
            app.show_policy_tip(Instant::now());
            apply_turn_start_completion(turn, None, conversation, thread_subscription, app);
        }
        Completion::Sessions(SessionCompletion::ManagerCreated(Err(error))) => {
            app.update(AppEvent::FailureReported(error));
        }
        Completion::Sessions(SessionCompletion::ThreadChanged(Ok(ConversationCompletion {
            conversation: next_conversation,
            change,
            subscription,
            switch,
        }))) => {
            *conversation = next_conversation;
            *thread_subscription = subscription;
            finish_conversation_change(
                conversation,
                app,
                change,
                switch,
                ConversationCompletionPresentation::Notice,
            );
        }
        Completion::Sessions(SessionCompletion::ThreadChanged(Err(error))) => {
            app.update(AppEvent::FailureReported(error));
        }
        Completion::Sessions(SessionCompletion::Changed {
            command,
            result:
                Ok(ConversationCompletion {
                    conversation: next_conversation,
                    change,
                    subscription,
                    switch,
                }),
        }) => {
            *conversation = next_conversation;
            *thread_subscription = subscription;
            finish_conversation_change(
                conversation,
                app,
                change,
                switch,
                ConversationCompletionPresentation::Command(command),
            );
        }
        Completion::Sessions(SessionCompletion::Changed {
            result: Err(error), ..
        })
        | Completion::ProductCommand(Err(error))
        | Completion::Presentation(Err(error)) => {
            app.update(AppEvent::FailureReported(error));
        }
        Completion::ProductCommand(Ok(ProductCommandCompletion {
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
                let command = output.command;
                finish_conversation_change(
                    conversation,
                    app,
                    change,
                    switch,
                    ConversationCompletionPresentation::Command(command),
                );
            }
        }
        Completion::Presentation(Ok(event)) => app.update(event),
        Completion::PreferredModelUpdated {
            command,
            result: Ok(update),
        } => {
            app.update(AppEvent::ModelSummaryReceived(update.summary));
            app.update(AppEvent::CommandCompleted {
                command,
                result: update.notice,
            });
            app.update(AppEvent::CommandPanelClosed);
        }
        Completion::PreferredModelUpdated {
            result: Err(error), ..
        } => app.update(AppEvent::FailureReported(error)),
        Completion::ThemeUpdated {
            command,
            label,
            theme,
            result: Ok(()),
        } => {
            app.update(AppEvent::RenderThemeChanged(theme));
            app.update(AppEvent::CommandCompleted {
                command,
                result: format!("Theme set to {label}"),
            });
        }
        Completion::ThemeUpdated {
            result: Err(error), ..
        } => app.update(AppEvent::FailureReported(error)),
        Completion::Skills(Ok(refresh)) => {
            app.replace_chat_input_catalog(refresh.input_catalog);
            if app.skills_view_is_active() {
                app.update(AppEvent::SkillSettingsUpdated(refresh.choices));
            } else {
                app.update(AppEvent::SkillDiagnosticsReceived(
                    refresh.choices.diagnostics,
                ));
            }
        }
        Completion::Skills(Err(error)) => {
            if app.skills_view_is_active() {
                app.update(AppEvent::FailureReported(error));
            }
        }
        Completion::Thread(ThreadCompletion::Started { result, .. }) => {
            apply_turn_start_completion(result, None, conversation, thread_subscription, app)
        }
        Completion::Thread(ThreadCompletion::QueuedTurnStarted {
            queue_id, result, ..
        }) => apply_turn_start_completion(
            result,
            Some(queue_id),
            conversation,
            thread_subscription,
            app,
        ),
        Completion::Thread(ThreadCompletion::Steered {
            source,
            steer_id,
            result: Ok((steer, snapshot)),
            ..
        }) => {
            app.update(AppEvent::SteerCompleted { source, steer_id });
            if snapshot.thread.sequence < conversation.thread_sequence().max(steer.sequence) {
                return;
            }
            conversation.set_thread_sequence(snapshot.thread.sequence.max(steer.sequence));
            let install_transcript = thread_subscription.apply_latest_snapshot(
                &snapshot.thread,
                snapshot.transcript.revision,
                snapshot.boundary,
            );
            apply_thread_snapshot_parts(
                app,
                snapshot.thread,
                install_transcript.then_some(snapshot.transcript),
            );
        }
        Completion::Thread(ThreadCompletion::Steered {
            source,
            steer_id,
            result: Err(error),
            ..
        }) => {
            app.update(AppEvent::SteerSubmissionFailed {
                source,
                steer_id,
                error: error.to_string(),
            });
        }
        Completion::Thread(ThreadCompletion::RequestResolved {
            request,
            result: Ok(snapshot),
            ..
        }) => {
            app.update(AppEvent::ThreadRequestResolved(request));
            if snapshot.thread.sequence < conversation.thread_sequence() {
                return;
            }
            conversation.set_thread_sequence(snapshot.thread.sequence);
            let install_transcript = thread_subscription.apply_latest_snapshot(
                &snapshot.thread,
                snapshot.transcript.revision,
                snapshot.boundary,
            );
            apply_thread_snapshot_parts(
                app,
                snapshot.thread,
                install_transcript.then_some(snapshot.transcript),
            );
        }
        Completion::Thread(ThreadCompletion::RequestResolved {
            request,
            result: Err(error),
            ..
        }) => {
            app.update(AppEvent::ThreadRequestSubmissionFailed {
                request,
                error: error.to_string(),
            });
        }
        Completion::Thread(ThreadCompletion::Refreshed {
            result: Ok(snapshot),
            ..
        }) => {
            if snapshot.thread.sequence < conversation.thread_sequence() {
                return;
            }
            conversation.set_thread_sequence(snapshot.thread.sequence);
            let install_transcript = thread_subscription.apply_latest_snapshot(
                &snapshot.thread,
                snapshot.transcript.revision,
                snapshot.boundary,
            );
            apply_thread_snapshot_parts(
                app,
                snapshot.thread,
                install_transcript.then_some(snapshot.transcript),
            );
        }
        Completion::Thread(ThreadCompletion::HistoryPage {
            result: Ok(page), ..
        }) => {
            thread_subscription.apply_history_page(&page.thread, page.boundary);
            app.update(AppEvent::ThreadTranscriptHistoryPageReceived(
                page.transcript,
            ));
        }
        Completion::Thread(ThreadCompletion::HistoryPage {
            result: Err(error), ..
        }) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        Completion::Thread(ThreadCompletion::Refreshed {
            result: Err(error), ..
        }) => {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
        Completion::Thread(ThreadCompletion::Interrupted {
            result: Ok(snapshot),
            ..
        }) => {
            if snapshot.thread.sequence < conversation.thread_sequence() {
                return;
            }
            conversation.set_thread_sequence(snapshot.thread.sequence);
            let install_transcript = thread_subscription.apply_latest_snapshot(
                &snapshot.thread,
                snapshot.transcript.revision,
                snapshot.boundary,
            );
            apply_thread_snapshot_parts(
                app,
                snapshot.thread,
                install_transcript.then_some(snapshot.transcript),
            );
        }
        Completion::Thread(ThreadCompletion::Interrupted {
            result: Err(error), ..
        }) => {
            app.update(AppEvent::InterruptFailed(error.to_string()));
        }
    }
}

pub(super) fn apply_tui_config(
    config: ConfigReadResult,
    model_catalog: Option<&zeta_app_server_protocol::protocol::model::ModelListResult>,
    app: &mut App,
) {
    match config::TerminalSettings::from_tui(&config.tui) {
        Ok(settings) => app.update(AppEvent::ConfigSettingsReceived(settings)),
        Err(error) => app.update(AppEvent::FailureReported(error)),
    }
    match keymap::settings_from_tui(&config.tui) {
        Ok(settings) => app.update(AppEvent::KeymapSettingsReceived(settings)),
        Err(error) => app.update(AppEvent::FailureReported(error)),
    }
    match StatusLineSettings::from_tui(&config.tui) {
        Ok(settings) => app.update(AppEvent::StatusLineSettingsReceived(settings)),
        Err(error) => app.update(AppEvent::FailureReported(error)),
    }
    app.update(AppEvent::ModelSummaryReceived(
        crate::models::ModelSummary::from_catalog(config.preferred_model, model_catalog),
    ));
}

fn report_turn_start_failure(app: &mut App, error: String) {
    if app.active_turn().is_some() {
        app.update(AppEvent::HostOperationCompleted(Err(format!(
            "could not start the Turn: {error}"
        ))));
    } else {
        app.update(AppEvent::FailureReported(error));
    }
}

fn apply_turn_start_completion(
    result: TurnStartCompletion,
    queue_id: Option<QueueId>,
    conversation: &mut ActiveConversation,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) {
    match result {
        TurnStartCompletion::Rejected(error) => {
            if let Some(queue_id) = queue_id {
                app.update(AppEvent::QueueSubmissionFailed {
                    queue_id,
                    error: error.to_string(),
                });
            } else {
                report_turn_start_failure(app, error.to_string());
            }
        }
        TurnStartCompletion::Accepted { start, snapshot } => {
            conversation.set_thread_sequence(start.sequence);
            app.set_active_turn_if_idle(start.turn_id);
            if let Some(queue_id) = queue_id {
                app.update(AppEvent::QueueSubmissionCompleted(queue_id));
            }
            match *snapshot {
                Ok(snapshot) => {
                    if snapshot.thread.sequence < conversation.thread_sequence() {
                        return;
                    }
                    conversation.set_thread_sequence(snapshot.thread.sequence.max(start.sequence));
                    let install_transcript = thread_subscription.apply_latest_snapshot(
                        &snapshot.thread,
                        snapshot.transcript.revision,
                        snapshot.boundary,
                    );
                    apply_thread_snapshot_parts(
                        app,
                        snapshot.thread,
                        install_transcript.then_some(snapshot.transcript),
                    );
                }
                Err(error) => {
                    app.update(AppEvent::HostOperationCompleted(Err(format!(
                        "Turn was accepted, but its updated snapshot could not be read: {error}"
                    ))));
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn apply_active_turn_snapshot(app: &mut App, turns: &[Turn]) {
    for update in app.sync_active_turn(turns) {
        apply_active_turn_update(app, update);
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
#[path = "completion_tests.rs"]
mod tests;

pub(super) fn apply_thread_snapshot(
    app: &mut App,
    snapshot: Thread,
    transcript: ThreadTranscriptSnapshot,
) {
    apply_thread_snapshot_parts(app, snapshot, Some(transcript));
}

fn apply_thread_snapshot_parts(
    app: &mut App,
    snapshot: Thread,
    transcript: Option<ThreadTranscriptSnapshot>,
) {
    app.update(AppEvent::ThreadContextChanged {
        session_id: snapshot.session_id.clone(),
        thread_id: snapshot.thread_id.clone(),
    });
    app.update(AppEvent::ThreadAccountingChanged {
        usage: snapshot.usage.clone(),
        reference_cost: snapshot.reference_cost.clone(),
    });
    app.update(AppEvent::ThreadGoalChanged(snapshot.goal.clone()));
    let active_turn_updates = app.sync_active_turn(&snapshot.turns);
    let active_turn = app.active_turn().cloned();
    let plan = turn_plan(active_turn.as_ref(), &snapshot.turns);
    let pending_interaction = active_turn.as_ref().and_then(|turn_id| {
        snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .and_then(|turn| {
                turn.pending_interaction
                    .as_ref()
                    .map(|pending| (turn.turn_id.clone(), pending.request_id.clone()))
            })
    });
    if let Some(transcript) = transcript {
        app.update(AppEvent::ThreadTranscriptSnapshotReceived(transcript));
    }
    app.update(AppEvent::TurnPlanChanged(plan));
    app.update(AppEvent::PendingInteractionChanged(pending_interaction));
    for update in active_turn_updates {
        apply_active_turn_update(app, update);
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

fn turn_plan(
    active_turn: Option<&TurnId>,
    turns: &[zeta_protocol::Turn],
) -> Option<zeta_protocol::PlanUpdate> {
    active_turn.and_then(|turn_id| {
        turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .and_then(|turn| turn.plan.clone())
    })
}

enum ConversationCompletionPresentation {
    Command(String),
    Notice,
    Silent,
}

fn finish_conversation_change(
    conversation: &mut ActiveConversation,
    app: &mut App,
    change: ConversationChange,
    switch: ThreadSwitch,
    presentation: ConversationCompletionPresentation,
) {
    if matches!(presentation, ConversationCompletionPresentation::Command(_)) {
        app.update(AppEvent::CommandPanelClosed);
    }
    if matches!(change.transcript, ConversationTranscript::Clear) {
        app.update(AppEvent::TranscriptCleared);
    }
    app.clear_active_turn();
    match switch {
        ThreadSwitch::Complete {
            snapshot,
            transcript,
        } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, snapshot, transcript);
        }
        ThreadSwitch::StaleSubscription {
            snapshot,
            transcript,
            error,
        } => {
            conversation.set_thread_sequence(snapshot.sequence);
            apply_thread_snapshot(app, snapshot, transcript);
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
        ConversationCompletionPresentation::Silent => {}
    }
}
