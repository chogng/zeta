use serde_json::Value;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_composer::Composer;
use zeta_composer::ComposerModelOption;
use zeta_input_classifier::InputConversation;
use zeta_input_classifier::InputHistoryEntry;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::TurnStatus;

/// Normalizes transport-owned model catalog entries into the Composer feature contract.
pub(crate) fn composer_model_options(entries: Vec<ModelCatalogEntry>) -> Vec<ComposerModelOption> {
    entries
        .into_iter()
        .map(|entry| ComposerModelOption {
            description: format!("{}/{}", entry.model.provider, entry.model.model),
            label: entry.display_name,
            model: entry.model,
        })
        .collect()
}

/// Projects canonical Thread state into the classifier's product-neutral context.
pub(crate) fn synchronize_composer_classifier(composer: &mut Composer, thread: &Thread) {
    composer.replace_classification_history(classification_history_for_thread(thread));
    let Some(turn) = thread.turns.last() else {
        composer.synchronize_conversation(InputConversation::Standalone);
        return;
    };
    let has_agent_message = turn
        .items
        .iter()
        .any(|item| matches!(item, ThreadItem::AgentMessage { .. }));
    match (turn.status, has_agent_message) {
        (TurnStatus::Completed, true) => {
            composer.synchronize_conversation(InputConversation::AgentFollowUp);
        }
        (
            TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
            | TurnStatus::Cancelling,
            true,
        ) => composer.mark_agent_response_started(),
        _ => composer.synchronize_conversation(InputConversation::Standalone),
    }
}

/// Applies transient and committed Turn events that change follow-up interpretation.
pub(crate) fn update_composer_classifier(composer: &mut Composer, update: &ThreadUpdate) {
    match update {
        ThreadUpdate::ItemStarted {
            item: ThreadItem::AgentMessage { .. },
            ..
        }
        | ThreadUpdate::Committed {
            event:
                ThreadEvent::ItemCompleted {
                    item: ThreadItem::AgentMessage { .. },
                    ..
                },
            ..
        } => composer.mark_agent_response_started(),
        ThreadUpdate::Committed {
            event: ThreadEvent::TurnCompleted { .. },
            ..
        } => composer.mark_agent_turn_completed(),
        ThreadUpdate::Committed {
            event: ThreadEvent::TurnFailed { .. } | ThreadEvent::TurnInterrupted { .. },
            ..
        } => composer.mark_agent_turn_ended_without_response(),
        _ => {}
    }
}

fn classification_history_for_thread(thread: &Thread) -> Vec<InputHistoryEntry> {
    thread
        .turns
        .iter()
        .flat_map(|turn| {
            let agent_prompts = turn.items.iter().filter_map(|item| match item {
                ThreadItem::UserMessage { text, .. } => {
                    Some(InputHistoryEntry::agent(text.clone()))
                }
                _ => None,
            });
            let has_agent_prompt = turn
                .items
                .iter()
                .any(|item| matches!(item, ThreadItem::UserMessage { .. }));
            let shell_command = (!has_agent_prompt && !turn_command_was_not_found(&turn.items))
                .then(|| turn.items.iter().find_map(shell_history_entry))
                .flatten();
            agent_prompts.chain(shell_command)
        })
        .collect()
}

fn shell_history_entry(item: &ThreadItem) -> Option<InputHistoryEntry> {
    let ThreadItem::ToolCall {
        name,
        arguments_json,
        ..
    } = item
    else {
        return None;
    };
    if name.as_str() != "shell-command" {
        return None;
    }
    let Value::Object(arguments) = serde_json::from_str(arguments_json).ok()? else {
        return None;
    };
    if let Some(command) = arguments.get("command").and_then(Value::as_str) {
        return Some(InputHistoryEntry::shell(command));
    }
    let arguments = arguments.get("arguments").and_then(Value::as_array)?;
    (arguments.len() == 2 && arguments[0].as_str() == Some("-lc"))
        .then(|| arguments[1].as_str().map(InputHistoryEntry::shell))
        .flatten()
}

fn turn_command_was_not_found(items: &[ThreadItem]) -> bool {
    items.iter().any(|item| {
        let ThreadItem::ToolResult { text, .. } = item else {
            return false;
        };
        serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|value| value.get("exit_code").and_then(Value::as_i64))
            == Some(127)
    })
}

#[cfg(test)]
#[path = "composer_host_tests.rs"]
mod tests;
