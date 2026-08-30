use super::exec_cell::ExecCell;
use crate::components::chat_history::CommandStatus;
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
use zeta_protocol::ToolCallId;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct TranscriptCellId(String);

impl TranscriptCellId {
    fn for_entry(entry_id: &str) -> Self {
        Self(format!("entry:{entry_id}"))
    }

    pub(super) fn for_tool_call(tool_call_id: &ToolCallId) -> Self {
        Self(format!("exec:{}", tool_call_id.as_str()))
    }

    fn local(kind: &str, sequence: u64) -> Self {
        Self(format!("local:{kind}:{sequence}"))
    }

    pub(crate) fn from_render_key(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CellLifecycle {
    Live,
    Final,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TranscriptCellBody {
    Message {
        role: MessageRole,
        text: String,
    },
    Reasoning(String),
    Exec(ExecCell),
    Plan(String),
    Error(String),
    Notice(String),
    Command {
        command: String,
        result: Option<String>,
        status: CommandStatus,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptCell {
    cell_id: TranscriptCellId,
    source_entry_id: Option<String>,
    lifecycle: CellLifecycle,
    body: TranscriptCellBody,
}

impl TranscriptCell {
    pub(crate) fn cell_id(&self) -> &TranscriptCellId {
        &self.cell_id
    }

    pub(crate) fn lifecycle(&self) -> CellLifecycle {
        match &self.body {
            TranscriptCellBody::Exec(exec) if exec.is_live() => CellLifecycle::Live,
            TranscriptCellBody::Exec(_) => CellLifecycle::Final,
            _ => self.lifecycle,
        }
    }

    pub(crate) fn can_expand(&self) -> bool {
        match &self.body {
            TranscriptCellBody::Exec(exec) => exec.can_expand(),
            TranscriptCellBody::Reasoning(text) | TranscriptCellBody::Error(text) => {
                text.lines().count() > 1 || text.chars().count() > 120
            }
            _ => false,
        }
    }

    pub(crate) fn has_details(&self) -> bool {
        match &self.body {
            TranscriptCellBody::Exec(exec) => exec.has_details(),
            TranscriptCellBody::Reasoning(text) | TranscriptCellBody::Error(text) => {
                text.lines().count() > 12
            }
            _ => false,
        }
    }

    pub(crate) fn details(&self) -> Option<String> {
        match &self.body {
            TranscriptCellBody::Exec(exec) if exec.has_details() => Some(exec.full_details()),
            TranscriptCellBody::Reasoning(text) | TranscriptCellBody::Error(text)
                if self.has_details() =>
            {
                Some(text.clone())
            }
            _ => None,
        }
    }

    fn view(&self, expanded: bool, selected: bool) -> Message {
        let message = match &self.body {
            TranscriptCellBody::Message { role, text } => {
                Message::plain(*role, text.clone()).with_cell_id(self.cell_id.as_str())
            }
            TranscriptCellBody::Reasoning(text) => {
                if expanded {
                    Message::plain(MessageRole::Reasoning, "Thought".into())
                        .with_detail(bounded_preview(text, 12))
                        .with_cell_id(self.cell_id.as_str())
                } else {
                    Message::plain(MessageRole::Reasoning, "Thought".into())
                        .with_cell_id(self.cell_id.as_str())
                }
            }
            TranscriptCellBody::Exec(exec) => exec.view(expanded),
            TranscriptCellBody::Plan(text) => {
                Message::plain(MessageRole::Plan, text.clone()).with_cell_id(self.cell_id.as_str())
            }
            TranscriptCellBody::Error(text) => {
                let summary = text.lines().next().unwrap_or("Error").to_owned();
                let message =
                    Message::plain(MessageRole::Error, summary).with_cell_id(self.cell_id.as_str());
                if expanded {
                    message.with_detail(bounded_preview(text, 12))
                } else {
                    message
                }
            }
            TranscriptCellBody::Notice(text) => Message::plain(MessageRole::Notice, text.clone())
                .with_cell_id(self.cell_id.as_str()),
            TranscriptCellBody::Command {
                command,
                result,
                status,
            } => Message::command(command.clone(), *status, result.clone())
                .with_cell_id(self.cell_id.as_str()),
        };
        message.with_cell_actions(self.can_expand(), expanded, self.has_details(), selected)
    }

    fn source_ids(&self) -> Vec<&str> {
        match &self.body {
            TranscriptCellBody::Exec(exec) => exec.source_ids(),
            _ => self.source_entry_id.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct TranscriptProjection {
    cells: Vec<TranscriptCell>,
    next_local_id: u64,
}

impl TranscriptProjection {
    pub(super) fn replace(&mut self, snapshot: ThreadTranscriptSnapshot) {
        self.cells.clear();
        for entry in snapshot.entries {
            self.upsert(entry);
        }
    }

    pub(super) fn prepend_history(&mut self, snapshot: ThreadTranscriptSnapshot) {
        let existing = self
            .cells
            .iter()
            .flat_map(TranscriptCell::source_ids)
            .collect::<BTreeSet<_>>();
        let mut older = Self::default();
        for entry in snapshot.entries {
            if !existing.contains(entry.entry_id()) {
                older.upsert(entry);
            }
        }
        older.cells.append(&mut self.cells);
        self.cells = older.cells;
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
        self.cells.clear();
    }

    pub(super) fn views(
        &self,
        expanded: &BTreeSet<TranscriptCellId>,
        selected: Option<&TranscriptCellId>,
    ) -> Vec<Message> {
        self.cells
            .iter()
            .map(|cell| {
                cell.view(
                    expanded.contains(cell.cell_id()),
                    selected == Some(cell.cell_id()),
                )
            })
            .collect()
    }

    pub(super) fn cells(&self) -> &[TranscriptCell] {
        &self.cells
    }

    pub(super) fn details(&self, cell_id: &TranscriptCellId) -> Option<String> {
        self.cells
            .iter()
            .find(|cell| cell.cell_id() == cell_id)
            .and_then(TranscriptCell::details)
    }

    pub(super) fn push_message(&mut self, role: MessageRole, text: String) {
        let cell_id = self.local_id("message");
        self.cells.push(TranscriptCell {
            cell_id,
            source_entry_id: None,
            lifecycle: CellLifecycle::Final,
            body: TranscriptCellBody::Message { role, text },
        });
    }

    pub(super) fn push_notice(&mut self, text: String) {
        let cell_id = self.local_id("notice");
        self.cells.push(TranscriptCell {
            cell_id,
            source_entry_id: None,
            lifecycle: CellLifecycle::Final,
            body: TranscriptCellBody::Notice(text),
        });
    }

    pub(super) fn push_error(&mut self, text: String) {
        let cell_id = self.local_id("error");
        self.cells.push(TranscriptCell {
            cell_id,
            source_entry_id: None,
            lifecycle: CellLifecycle::Final,
            body: TranscriptCellBody::Error(text),
        });
    }

    pub(super) fn command_started(&mut self, command: String) {
        let cell_id = self.local_id("command");
        self.cells.push(TranscriptCell {
            cell_id,
            source_entry_id: None,
            lifecycle: CellLifecycle::Live,
            body: TranscriptCellBody::Command {
                command,
                result: None,
                status: CommandStatus::Running,
            },
        });
    }

    pub(super) fn command_completed(&mut self, command: String, result: String) {
        if let Some(cell) = self.cells.iter_mut().rev().find(|cell| {
            matches!(
                &cell.body,
                TranscriptCellBody::Command {
                    command: active,
                    status: CommandStatus::Running,
                    ..
                } if active == &command
            )
        }) {
            cell.lifecycle = CellLifecycle::Final;
            cell.body = TranscriptCellBody::Command {
                command,
                result: Some(result),
                status: CommandStatus::Succeeded,
            };
            return;
        }
        let cell_id = self.local_id("command");
        self.cells.push(TranscriptCell {
            cell_id,
            source_entry_id: None,
            lifecycle: CellLifecycle::Final,
            body: TranscriptCellBody::Command {
                command,
                result: Some(result),
                status: CommandStatus::Succeeded,
            },
        });
    }

    fn upsert(&mut self, entry: ThreadTranscriptEntry) {
        match entry {
            ThreadTranscriptEntry::Item {
                entry_id,
                item:
                    ThreadItem::ToolCall {
                        tool_call_id,
                        name,
                        arguments_json,
                        ..
                    },
                ..
            } => self.upsert_tool_call(entry_id, tool_call_id, name, pretty_json(&arguments_json)),
            ThreadTranscriptEntry::Item {
                entry_id,
                item:
                    ThreadItem::ToolResult {
                        tool_call_id,
                        text,
                        is_error,
                        ..
                    },
                ..
            } => self.complete_tool(entry_id, tool_call_id, text, is_error),
            ThreadTranscriptEntry::ToolOutput {
                entry_id,
                tool_call_id,
                stream,
                text,
                ..
            } => self.apply_tool_output(entry_id, tool_call_id, stream, text),
            entry => self.upsert_regular(cell_from_entry(&entry)),
        }
    }

    fn upsert_regular(&mut self, cell: TranscriptCell) {
        if let Some(existing) = self
            .cells
            .iter_mut()
            .find(|existing| existing.source_entry_id == cell.source_entry_id)
        {
            *existing = cell;
        } else {
            self.cells.push(cell);
        }
    }

    fn upsert_tool_call(
        &mut self,
        entry_id: String,
        tool_call_id: ToolCallId,
        name: zeta_protocol::ToolName,
        arguments: String,
    ) {
        if let Some(exec) = self.exec_for_call_mut(&tool_call_id) {
            exec.update_call(entry_id, &tool_call_id, &name, arguments);
            return;
        }
        if let Some(TranscriptCell {
            body: TranscriptCellBody::Exec(exec),
            ..
        }) = self.cells.last_mut()
            && exec.can_accept(&name)
        {
            exec.push_call(entry_id, tool_call_id, &name, arguments);
            return;
        }
        self.cells.push(TranscriptCell {
            cell_id: TranscriptCellId::for_tool_call(&tool_call_id),
            source_entry_id: None,
            lifecycle: CellLifecycle::Live,
            body: TranscriptCellBody::Exec(ExecCell::start(
                entry_id,
                tool_call_id,
                &name,
                arguments,
            )),
        });
    }

    fn apply_tool_output(
        &mut self,
        entry_id: String,
        tool_call_id: ToolCallId,
        stream: zeta_protocol::ToolOutputStream,
        text: String,
    ) {
        if self.exec_for_call_mut(&tool_call_id).is_none() {
            self.cells.push(TranscriptCell {
                cell_id: TranscriptCellId::for_tool_call(&tool_call_id),
                source_entry_id: None,
                lifecycle: CellLifecycle::Live,
                body: TranscriptCellBody::Exec(ExecCell::recovered(tool_call_id.clone())),
            });
        }
        self.exec_for_call_mut(&tool_call_id)
            .expect("the recovered ExecCell owns the ToolCall")
            .apply_output(entry_id, &tool_call_id, stream, text);
    }

    fn complete_tool(
        &mut self,
        entry_id: String,
        tool_call_id: ToolCallId,
        result: String,
        failed: bool,
    ) {
        if self.exec_for_call_mut(&tool_call_id).is_none() {
            self.cells.push(TranscriptCell {
                cell_id: TranscriptCellId::for_tool_call(&tool_call_id),
                source_entry_id: None,
                lifecycle: CellLifecycle::Final,
                body: TranscriptCellBody::Exec(ExecCell::recovered(tool_call_id.clone())),
            });
        }
        self.exec_for_call_mut(&tool_call_id)
            .expect("the recovered ExecCell owns the ToolCall")
            .complete(entry_id, &tool_call_id, result, failed);
    }

    fn exec_for_call_mut(&mut self, tool_call_id: &ToolCallId) -> Option<&mut ExecCell> {
        self.cells.iter_mut().find_map(|cell| match &mut cell.body {
            TranscriptCellBody::Exec(exec) if exec.contains_call(tool_call_id) => Some(exec),
            _ => None,
        })
    }

    fn remove(&mut self, entry_ids: &[String]) {
        for entry_id in entry_ids {
            for cell in &mut self.cells {
                if let TranscriptCellBody::Exec(exec) = &mut cell.body
                    && exec.contains_source(entry_id)
                {
                    exec.remove_entry(entry_id);
                }
            }
            self.cells.retain(|cell| {
                cell.source_entry_id.as_deref() != Some(entry_id)
                    || matches!(&cell.body, TranscriptCellBody::Exec(exec) if !exec.is_empty())
            });
        }
        self.cells.retain(
            |cell| !matches!(&cell.body, TranscriptCellBody::Exec(exec) if exec.is_empty()),
        );
    }

    fn clear_transient(&mut self) {
        for cell in &mut self.cells {
            if let TranscriptCellBody::Exec(exec) = &mut cell.body {
                exec.clear_live();
            }
        }
        self.cells.retain(|cell| {
            cell.lifecycle() == CellLifecycle::Final
                && !matches!(&cell.body, TranscriptCellBody::Exec(exec) if exec.is_empty())
        });
    }

    fn local_id(&mut self, kind: &str) -> TranscriptCellId {
        self.next_local_id = self.next_local_id.saturating_add(1);
        TranscriptCellId::local(kind, self.next_local_id)
    }
}

fn cell_from_entry(entry: &ThreadTranscriptEntry) -> TranscriptCell {
    let entry_id = entry.entry_id().to_owned();
    let lifecycle = if entry.is_transient() {
        CellLifecycle::Live
    } else {
        CellLifecycle::Final
    };
    let body = match entry {
        ThreadTranscriptEntry::Item { item, .. } => match item {
            ThreadItem::UserMessage { text, .. } => TranscriptCellBody::Message {
                role: MessageRole::User,
                text: text.clone(),
            },
            ThreadItem::UserContext { name, content, .. } => TranscriptCellBody::Message {
                role: MessageRole::User,
                text: format!("Context · {name}\n{content}"),
            },
            ThreadItem::UserImage { .. } | ThreadItem::UserImageAttachment { .. } => {
                TranscriptCellBody::Message {
                    role: MessageRole::User,
                    text: "[Image]".into(),
                }
            }
            ThreadItem::AgentMessage { text, .. } => TranscriptCellBody::Message {
                role: MessageRole::Agent,
                text: text.clone(),
            },
            ThreadItem::Reasoning { text, .. } => TranscriptCellBody::Reasoning(text.clone()),
            ThreadItem::Plan { text, .. } => TranscriptCellBody::Plan(text.clone()),
            ThreadItem::ToolCall { .. } | ThreadItem::ToolResult { .. } => {
                unreachable!("Tool entries are routed into ExecCell")
            }
        },
        ThreadTranscriptEntry::TurnPlan { plan, .. } => {
            TranscriptCellBody::Plan(present_plan(plan))
        }
        ThreadTranscriptEntry::TurnError { error, .. } => {
            TranscriptCellBody::Error(error.message.clone())
        }
        ThreadTranscriptEntry::ToolOutput { .. } => {
            unreachable!("Tool output is routed into ExecCell")
        }
    };
    TranscriptCell {
        cell_id: TranscriptCellId::for_entry(&entry_id),
        source_entry_id: Some(entry_id),
        lifecycle,
        body,
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

fn bounded_preview(text: &str, max_lines: usize) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() <= max_lines {
        return text.to_owned();
    }
    let omitted = lines.len().saturating_sub(max_lines);
    format!(
        "{}\n… {omitted} lines omitted",
        lines[..max_lines].join("\n")
    )
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
