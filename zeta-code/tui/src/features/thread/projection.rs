use crate::components::transcript::Message;
use crate::components::transcript::MessageRole;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use zeta_protocol::ItemDelta;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;
use zeta_protocol::Thread;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolOutputStream;

const MAX_TRANSIENT_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_TRANSIENT_MESSAGES: usize = 1_024;
const TRANSIENT_TRUNCATION_MARKER: &str = "\n… transient output truncated …";

#[derive(Debug, Default)]
pub(super) struct TransientProjection {
    source_ids: BTreeSet<String>,
    source_order: VecDeque<String>,
}

impl TransientProjection {
    pub(super) fn clear(&mut self) {
        self.source_ids.clear();
        self.source_order.clear();
    }

    pub(super) fn remove_from(&mut self, messages: &mut Vec<Message>) {
        messages.retain(|message| {
            message
                .source_id
                .as_ref()
                .is_none_or(|source_id| !self.source_ids.contains(source_id))
        });
        self.clear();
    }

    fn contains(&self, source_id: &str) -> bool {
        self.source_ids.contains(source_id)
    }

    fn reserve(&mut self, messages: &mut Vec<Message>, source_id: &str) {
        if self.source_ids.contains(source_id) {
            return;
        }
        while self.source_ids.len() >= MAX_TRANSIENT_MESSAGES {
            let Some(expired) = self.source_order.pop_front() else {
                break;
            };
            self.source_ids.remove(&expired);
            messages.retain(|message| message.source_id.as_deref() != Some(expired.as_str()));
        }
        self.source_ids.insert(source_id.to_owned());
        self.source_order.push_back(source_id.to_owned());
    }
}

pub(super) fn project_messages(thread: &Thread) -> Vec<Message> {
    let mut messages = Vec::new();
    for turn in &thread.turns {
        let tool_names = turn
            .items
            .iter()
            .filter_map(|item| match item {
                ThreadItem::ToolCall {
                    tool_call_id, name, ..
                } => Some((tool_call_id.clone(), name.as_str().to_owned())),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        messages.extend(
            turn.items
                .iter()
                .map(|item| project_item(item, &tool_names)),
        );
        if let Some(plan) = &turn.plan {
            messages.push(
                Message::plain(MessageRole::Plan, present_plan(plan))
                    .with_source_id(plan_source_id(turn.turn_id.as_str())),
            );
        }
    }
    messages
}

pub(super) fn apply_transient(
    messages: &mut Vec<Message>,
    transient: &mut TransientProjection,
    envelope: &ThreadUpdateEnvelope,
) {
    match &envelope.update {
        ThreadUpdate::Committed { .. } => {}
        ThreadUpdate::ItemStarted { item, .. } => upsert_started_item(messages, transient, item),
        ThreadUpdate::ItemDelta { item_id, delta, .. } => {
            let source_id = item_source_id(item_id.as_str());
            let (role, text) = match delta {
                ItemDelta::AgentMessage { text } => (MessageRole::Agent, text),
                ItemDelta::Reasoning { text } => (MessageRole::Reasoning, text),
                ItemDelta::Plan { text } => (MessageRole::Plan, text),
            };
            append_or_insert(messages, transient, source_id, role, text);
        }
        ThreadUpdate::ToolOutputDelta {
            turn_id,
            tool_call_id,
            stream,
            text,
        } => append_tool_output(
            messages,
            transient,
            turn_id.as_str(),
            tool_call_id,
            *stream,
            text,
        ),
    }
}

fn upsert_started_item(
    messages: &mut Vec<Message>,
    transient: &mut TransientProjection,
    item: &ThreadItem,
) {
    let source_id = source_id_for_item(item);
    if message_by_source_mut(messages, &source_id).is_none() {
        transient.reserve(messages, &source_id);
        messages.push(bound_transient_message(project_item(
            item,
            &BTreeMap::new(),
        )));
    }
}

fn append_or_insert(
    messages: &mut Vec<Message>,
    transient: &mut TransientProjection,
    source_id: String,
    role: MessageRole,
    text: &str,
) {
    if transient.contains(&source_id) {
        if let Some(message) = message_by_source_mut(messages, &source_id) {
            append_bounded(&mut message.text, text);
        }
    } else if message_by_source_mut(messages, &source_id).is_none() {
        transient.reserve(messages, &source_id);
        messages.push(Message::plain(role, bounded_transient_text(text)).with_source_id(source_id));
    }
}

fn append_tool_output(
    messages: &mut Vec<Message>,
    transient: &mut TransientProjection,
    turn_id: &str,
    tool_call_id: &ToolCallId,
    stream: ToolOutputStream,
    text: &str,
) {
    let stream_label = match stream {
        ToolOutputStream::Stdout => "stdout",
        ToolOutputStream::Stderr => "stderr",
    };
    let source_id = format!("tool-output:{turn_id}:{tool_call_id}:{stream_label}");
    if transient.contains(&source_id) {
        if let Some(message) = message_by_source_mut(messages, &source_id) {
            append_bounded(message.detail.get_or_insert_with(String::new), text);
        }
        return;
    }
    if message_by_source_mut(messages, &source_id).is_some() {
        return;
    }
    let tool_label = messages
        .iter()
        .find(|message| message.source_id.as_deref() == Some(&tool_call_source_id(tool_call_id)))
        .map(|message| message.text.clone())
        .unwrap_or_else(|| "Tool".into());
    let role = match stream {
        ToolOutputStream::Stdout => MessageRole::Tool,
        ToolOutputStream::Stderr => MessageRole::ToolError,
    };
    transient.reserve(messages, &source_id);
    messages.push(
        Message::plain(role, format!("{tool_label} · {stream_label}"))
            .with_detail(bounded_transient_text(text))
            .with_source_id(source_id),
    );
}

fn bound_transient_message(mut message: Message) -> Message {
    message.text = bounded_transient_text(&message.text);
    message.detail = message.detail.as_deref().map(bounded_transient_text);
    message
}

fn bounded_transient_text(text: &str) -> String {
    let mut bounded = String::new();
    append_bounded(&mut bounded, text);
    bounded
}

fn append_bounded(target: &mut String, addition: &str) {
    if target.ends_with(TRANSIENT_TRUNCATION_MARKER) {
        return;
    }
    let content_limit = MAX_TRANSIENT_MESSAGE_BYTES - TRANSIENT_TRUNCATION_MARKER.len();
    if target.len() >= content_limit {
        truncate_to_char_boundary(target, content_limit);
        target.push_str(TRANSIENT_TRUNCATION_MARKER);
        return;
    }
    let available = content_limit - target.len();
    if addition.len() <= available {
        target.push_str(addition);
        return;
    }
    let end = char_boundary_at_or_before(addition, available);
    target.push_str(&addition[..end]);
    target.push_str(TRANSIENT_TRUNCATION_MARKER);
}

fn truncate_to_char_boundary(value: &mut String, maximum: usize) {
    let end = char_boundary_at_or_before(value, maximum);
    value.truncate(end);
}

fn char_boundary_at_or_before(value: &str, maximum: usize) -> usize {
    let mut end = maximum.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn project_item(item: &ThreadItem, tool_names: &BTreeMap<ToolCallId, String>) -> Message {
    match item {
        ThreadItem::UserMessage { item_id, text, .. } => {
            item_message(item_id.as_str(), MessageRole::User, text.clone())
        }
        ThreadItem::UserImage { item_id, .. } | ThreadItem::UserImageAttachment { item_id, .. } => {
            item_message(item_id.as_str(), MessageRole::User, "[Image]".into())
        }
        ThreadItem::AgentMessage { item_id, text, .. } => {
            item_message(item_id.as_str(), MessageRole::Agent, text.clone())
        }
        ThreadItem::Reasoning { item_id, text, .. } => {
            item_message(item_id.as_str(), MessageRole::Reasoning, text.clone())
        }
        ThreadItem::Plan { item_id, text, .. } => {
            item_message(item_id.as_str(), MessageRole::Plan, text.clone())
        }
        ThreadItem::ToolCall {
            tool_call_id,
            name,
            arguments_json,
            ..
        } => Message::plain(MessageRole::Tool, format!("Tool · {name}"))
            .with_detail(pretty_json(arguments_json))
            .with_source_id(tool_call_source_id(tool_call_id)),
        ThreadItem::ToolResult {
            tool_call_id,
            text,
            is_error,
            ..
        } => {
            let label = tool_names
                .get(tool_call_id)
                .map(String::as_str)
                .unwrap_or(tool_call_id.as_str());
            Message::plain(
                if *is_error {
                    MessageRole::ToolError
                } else {
                    MessageRole::Tool
                },
                format!("Tool result · {label}"),
            )
            .with_detail(text)
            .with_source_id(format!("tool-result:{tool_call_id}"))
        }
    }
}

fn item_message(item_id: &str, role: MessageRole, text: String) -> Message {
    Message::plain(role, text).with_source_id(item_source_id(item_id))
}

fn source_id_for_item(item: &ThreadItem) -> String {
    match item {
        ThreadItem::ToolCall { tool_call_id, .. } => tool_call_source_id(tool_call_id),
        ThreadItem::ToolResult { tool_call_id, .. } => format!("tool-result:{tool_call_id}"),
        _ => item_source_id(item.item_id().as_str()),
    }
}

fn item_source_id(item_id: &str) -> String {
    format!("item:{item_id}")
}

fn plan_source_id(turn_id: &str) -> String {
    format!("plan-update:{turn_id}")
}

fn tool_call_source_id(tool_call_id: &ToolCallId) -> String {
    format!("tool-call:{tool_call_id}")
}

fn message_by_source_mut<'a>(
    messages: &'a mut [Message],
    source_id: &str,
) -> Option<&'a mut Message> {
    messages
        .iter_mut()
        .find(|message| message.source_id.as_deref() == Some(source_id))
}

fn pretty_json(value: &str) -> String {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| value.to_owned())
}

fn present_plan(plan: &PlanUpdate) -> String {
    let mut lines = plan.explanation.iter().cloned().collect::<Vec<_>>();
    lines.extend(plan.steps.iter().map(|step| {
        let marker = match step.status {
            PlanStepStatus::Pending => "[ ]",
            PlanStepStatus::InProgress => "[>]",
            PlanStepStatus::Completed => "[x]",
        };
        format!("{marker} {}", step.step)
    }));
    if lines.is_empty() {
        "Plan updated".into()
    } else {
        lines.join("\n")
    }
}
