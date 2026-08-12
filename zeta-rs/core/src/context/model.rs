use super::ContextBudget;
use crate::ThreadSnapshot;
use std::collections::BTreeMap;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ItemId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolDefinition;
use zeta_protocol::TurnId;

/// The semantic precedence of one instruction fragment.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum InstructionLayer {
    System,
    Product,
    Workspace,
    Skill,
}

/// Whether budget pressure may remove an instruction fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstructionRetention {
    Required,
    BestEffort,
}

/// Stable, diagnostic provenance for an instruction fragment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InstructionSource {
    kind: &'static str,
    identity: String,
    revision: String,
}

impl InstructionSource {
    pub(crate) fn new(
        kind: &'static str,
        identity: impl Into<String>,
        revision: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            identity: identity.into(),
            revision: revision.into(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        self.kind
    }

    pub(crate) fn identity(&self) -> &str {
        &self.identity
    }

    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }
}

/// One bounded instruction contribution before precedence and budget resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InstructionFragment {
    source: InstructionSource,
    layer: InstructionLayer,
    retention: InstructionRetention,
    body: String,
}

impl InstructionFragment {
    pub(crate) fn new(
        source: InstructionSource,
        layer: InstructionLayer,
        retention: InstructionRetention,
        body: impl Into<String>,
    ) -> Self {
        Self {
            source,
            layer,
            retention,
            body: body.into(),
        }
    }

    pub(crate) fn source(&self) -> &InstructionSource {
        &self.source
    }

    pub(crate) const fn layer(&self) -> InstructionLayer {
        self.layer
    }

    pub(crate) const fn retention(&self) -> InstructionRetention {
        self.retention
    }

    pub(crate) fn body(&self) -> &str {
        &self.body
    }
}

/// Complete immutable input to one context-planning operation.
#[derive(Clone, Debug)]
pub(crate) struct ContextInput {
    source_thread_sequence: u64,
    current_turn_id: TurnId,
    instructions: Vec<InstructionFragment>,
    items: Vec<ThreadItem>,
    checkpoints: Vec<ContextCheckpoint>,
    item_sequences: BTreeMap<ItemId, u64>,
    tools: Vec<ToolDefinition>,
    budget: ContextBudget,
}

impl ContextInput {
    pub(crate) fn new(
        snapshot: &ThreadSnapshot,
        current_turn_id: TurnId,
        instructions: Vec<InstructionFragment>,
        tools: Vec<ToolDefinition>,
        budget: ContextBudget,
    ) -> Self {
        Self {
            source_thread_sequence: snapshot.sequence,
            current_turn_id,
            instructions,
            items: snapshot.items.clone(),
            checkpoints: snapshot.context_checkpoints.clone(),
            item_sequences: snapshot.item_sequences.clone(),
            tools,
            budget,
        }
    }

    pub(crate) const fn source_thread_sequence(&self) -> u64 {
        self.source_thread_sequence
    }

    pub(crate) fn current_turn_id(&self) -> &TurnId {
        &self.current_turn_id
    }

    pub(crate) fn instructions(&self) -> &[InstructionFragment] {
        &self.instructions
    }

    pub(crate) fn items(&self) -> &[ThreadItem] {
        &self.items
    }

    pub(crate) fn checkpoints(&self) -> &[ContextCheckpoint] {
        &self.checkpoints
    }

    pub(crate) fn item_sequence(&self, item_id: &ItemId) -> Option<u64> {
        self.item_sequences.get(item_id).copied()
    }

    pub(crate) fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    pub(crate) const fn budget(&self) -> ContextBudget {
        self.budget
    }
}
