use std::collections::BTreeMap;

use zeta_protocol::{
    ItemDelta, ItemId, PlanUpdate, StreamCursor, Thread, ThreadItem, ThreadUpdate,
    ThreadUpdateEnvelope, ToolCallId, ToolOutputStream,
};

/// Result of applying one App Server Thread update to the native projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ThreadProjectionUpdate {
    Applied,
    Ignored,
    ResubscribeRequired,
}

/// Rebuildable native projection of one canonical Agent Thread.
///
/// Durable state is supplied by the App Server session adapter's authoritative snapshot/update
/// handoff. This projection only owns transient streaming text and presentation ordering between
/// authoritative snapshots; it is not a Thread transport or reducer.
#[derive(Default)]
pub(crate) struct ThreadProjection {
    thread: Option<Thread>,
    transient_items: BTreeMap<ItemId, ThreadItem>,
    transient_tool_outputs: BTreeMap<ToolCallId, TransientToolOutput>,
    stream_cursor: Option<StreamCursor>,
}

impl ThreadProjection {
    pub(crate) fn replace_snapshot(&mut self, thread: Thread) {
        self.thread = Some(thread);
        self.clear_transient();
    }

    pub(crate) fn thread(&self) -> Option<&Thread> {
        self.thread.as_ref()
    }

    pub(crate) fn plan(&self) -> Option<&PlanUpdate> {
        self.thread.as_ref()?.turns.last()?.plan.as_ref()
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = &ThreadItem> {
        self.thread
            .iter()
            .flat_map(|thread| thread.turns.iter())
            .flat_map(|turn| turn.items.iter())
            .chain(self.transient_items.values())
    }

    pub(crate) fn tool_output(&self, tool_call_id: &ToolCallId) -> Option<(&str, &str)> {
        self.transient_tool_outputs
            .get(tool_call_id)
            .map(|output| (output.stdout.as_str(), output.stderr.as_str()))
    }

    pub(crate) fn apply_update(
        &mut self,
        envelope: ThreadUpdateEnvelope,
    ) -> ThreadProjectionUpdate {
        let Some(thread) = self.thread.as_ref() else {
            return ThreadProjectionUpdate::ResubscribeRequired;
        };
        if envelope.thread_id != thread.thread_id || envelope.session_id != thread.session_id {
            return ThreadProjectionUpdate::Ignored;
        }
        if envelope.durable_sequence < thread.sequence {
            return ThreadProjectionUpdate::Ignored;
        }
        if envelope.durable_sequence > thread.sequence {
            self.clear_transient();
            return ThreadProjectionUpdate::ResubscribeRequired;
        }
        if !self.accept_stream_cursor(envelope.stream_cursor.as_ref()) {
            self.clear_transient();
            return ThreadProjectionUpdate::ResubscribeRequired;
        }
        match envelope.update {
            ThreadUpdate::Committed { .. } => {
                self.clear_transient();
                ThreadProjectionUpdate::ResubscribeRequired
            }
            ThreadUpdate::ItemStarted { item, .. } => {
                self.transient_items.insert(item.item_id().clone(), item);
                ThreadProjectionUpdate::Applied
            }
            ThreadUpdate::ItemDelta { item_id, delta, .. } => {
                let Some(item) = self.transient_items.get_mut(&item_id) else {
                    self.clear_transient();
                    return ThreadProjectionUpdate::ResubscribeRequired;
                };
                if apply_item_delta(item, delta) {
                    ThreadProjectionUpdate::Applied
                } else {
                    self.clear_transient();
                    ThreadProjectionUpdate::ResubscribeRequired
                }
            }
            ThreadUpdate::ToolOutputDelta {
                tool_call_id,
                stream,
                text,
                ..
            } => {
                let output = self.transient_tool_outputs.entry(tool_call_id).or_default();
                match stream {
                    ToolOutputStream::Stdout => output.stdout.push_str(&text),
                    ToolOutputStream::Stderr => output.stderr.push_str(&text),
                }
                ThreadProjectionUpdate::Applied
            }
        }
    }

    fn accept_stream_cursor(&mut self, next: Option<&StreamCursor>) -> bool {
        let Some(next) = next else {
            return true;
        };
        let Some(current) = self.stream_cursor.as_ref() else {
            self.stream_cursor = Some(next.clone());
            return true;
        };
        if next.stream_instance_id != current.stream_instance_id
            || next.sequence != current.sequence.saturating_add(1)
        {
            return false;
        }
        self.stream_cursor = Some(next.clone());
        true
    }

    fn clear_transient(&mut self) {
        self.transient_items.clear();
        self.transient_tool_outputs.clear();
        self.stream_cursor = None;
    }
}

#[derive(Default)]
struct TransientToolOutput {
    stdout: String,
    stderr: String,
}

fn apply_item_delta(item: &mut ThreadItem, delta: ItemDelta) -> bool {
    match (item, delta) {
        (ThreadItem::AgentMessage { text, .. }, ItemDelta::AgentMessage { text: delta })
        | (ThreadItem::Reasoning { text, .. }, ItemDelta::Reasoning { text: delta })
        | (ThreadItem::Plan { text, .. }, ItemDelta::Plan { text: delta }) => {
            text.push_str(&delta);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "thread_projection_tests.rs"]
mod tests;
