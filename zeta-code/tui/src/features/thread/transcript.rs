use crate::components::chat_history::Message;
use crate::components::chat_history::MessageRole;
use std::collections::BTreeSet;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptChange;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolOutputStream;

#[derive(Debug, Default)]
pub(super) struct TranscriptMessages {
    pub(super) messages: Vec<Message>,
    transient_ids: BTreeSet<String>,
}

impl TranscriptMessages {
    pub(super) fn replace(&mut self, snapshot: ThreadTranscriptSnapshot) {
        self.messages = snapshot.entries.iter().map(render_entry).collect();
        self.transient_ids = snapshot
            .entries
            .iter()
            .filter(|entry| entry.is_transient())
            .map(|entry| entry.entry_id().to_owned())
            .collect();
    }

    pub(super) fn prepend_history(&mut self, snapshot: ThreadTranscriptSnapshot) {
        let existing = self
            .messages
            .iter()
            .filter_map(|message| message.source_id.as_deref())
            .collect::<BTreeSet<_>>();
        let mut older = snapshot
            .entries
            .iter()
            .filter(|entry| !existing.contains(entry.entry_id()))
            .map(render_entry)
            .collect::<Vec<_>>();
        older.append(&mut self.messages);
        self.messages = older;
    }

    pub(super) fn apply(&mut self, update: ThreadTranscriptUpdateEnvelope) {
        for change in update.changes {
            match change {
                ThreadTranscriptChange::Upsert { entry } => self.upsert(entry),
                ThreadTranscriptChange::Remove { entry_ids } => self.remove(&entry_ids),
                ThreadTranscriptChange::ClearTransient => self.clear_transient(),
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.messages.clear();
        self.transient_ids.clear();
    }

    fn upsert(&mut self, entry: ThreadTranscriptEntry) {
        let entry_id = entry.entry_id().to_owned();
        if entry.is_transient() {
            self.transient_ids.insert(entry_id.clone());
        } else {
            self.transient_ids.remove(&entry_id);
        }
        let message = render_entry(&entry);
        if let Some(existing) = self
            .messages
            .iter_mut()
            .find(|message| message.source_id.as_deref() == Some(entry_id.as_str()))
        {
            *existing = message;
        } else {
            self.messages.push(message);
        }
    }

    fn remove(&mut self, entry_ids: &[String]) {
        let entry_ids = entry_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.messages.retain(|message| {
            message
                .source_id
                .as_deref()
                .is_none_or(|entry_id| !entry_ids.contains(entry_id))
        });
        self.transient_ids
            .retain(|entry_id| !entry_ids.contains(entry_id.as_str()));
    }

    fn clear_transient(&mut self) {
        self.messages.retain(|message| {
            message
                .source_id
                .as_ref()
                .is_none_or(|entry_id| !self.transient_ids.contains(entry_id))
        });
        self.transient_ids.clear();
    }
}

fn render_entry(entry: &ThreadTranscriptEntry) -> Message {
    match entry {
        ThreadTranscriptEntry::Item { entry_id, item, .. } => {
            render_item(item).with_source_id(entry_id)
        }
        ThreadTranscriptEntry::TurnPlan { entry_id, plan, .. } => {
            Message::plain(MessageRole::Plan, present_plan(plan)).with_source_id(entry_id)
        }
        ThreadTranscriptEntry::TurnError {
            entry_id, error, ..
        } => Message::plain(MessageRole::Error, error.message.clone()).with_source_id(entry_id),
        ThreadTranscriptEntry::ToolOutput {
            entry_id,
            stream,
            text,
            ..
        } => {
            let (role, label) = match stream {
                ToolOutputStream::Stdout => (MessageRole::Tool, "Tool · stdout"),
                ToolOutputStream::Stderr => (MessageRole::ToolError, "Tool · stderr"),
            };
            Message::plain(role, label.into())
                .with_detail(text)
                .with_source_id(entry_id)
        }
    }
}

fn render_item(item: &ThreadItem) -> Message {
    match item {
        ThreadItem::UserMessage { text, .. } => Message::plain(MessageRole::User, text.clone()),
        ThreadItem::UserContext { name, content, .. } => {
            Message::plain(MessageRole::User, format!("Context · {name}\n{content}"))
        }
        ThreadItem::UserImage { .. } | ThreadItem::UserImageAttachment { .. } => {
            Message::plain(MessageRole::User, "[Image]".into())
        }
        ThreadItem::AgentMessage { text, .. } => Message::plain(MessageRole::Agent, text.clone()),
        ThreadItem::Reasoning { text, .. } => Message::plain(MessageRole::Reasoning, text.clone()),
        ThreadItem::Plan { text, .. } => Message::plain(MessageRole::Plan, text.clone()),
        ThreadItem::ToolCall {
            name,
            arguments_json,
            ..
        } => Message::plain(MessageRole::Tool, format!("Tool · {name}"))
            .with_detail(pretty_json(arguments_json)),
        ThreadItem::ToolResult { text, is_error, .. } => Message::plain(
            if *is_error {
                MessageRole::ToolError
            } else {
                MessageRole::Tool
            },
            "Tool result".into(),
        )
        .with_detail(text),
    }
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
