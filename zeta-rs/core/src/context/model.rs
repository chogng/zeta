use super::ContextBudget;
use crate::ContextEvidence;
use crate::ThreadSnapshot;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
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
    kind: String,
    identity: String,
    revision: String,
}

impl InstructionSource {
    pub(crate) fn new(
        kind: impl Into<String>,
        identity: impl Into<String>,
        revision: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            identity: identity.into(),
            revision: revision.into(),
        }
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    pub(crate) fn identity(&self) -> &str {
        &self.identity
    }

    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }
}

impl TryFrom<&zeta_extension_api::PromptFragmentSource> for InstructionSource {
    type Error = crate::CoreError;

    fn try_from(source: &zeta_extension_api::PromptFragmentSource) -> Result<Self, Self::Error> {
        if source.kind().trim().is_empty()
            || source.identity().trim().is_empty()
            || source.revision().trim().is_empty()
        {
            return Err(crate::CoreError::Context(
                "extension prompt fragment provenance must not be empty".into(),
            ));
        }
        Ok(Self {
            kind: source.kind().to_owned(),
            identity: source.identity().to_owned(),
            revision: source.revision().to_owned(),
        })
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

impl TryFrom<zeta_extension_api::PromptFragment> for InstructionFragment {
    type Error = crate::CoreError;

    fn try_from(fragment: zeta_extension_api::PromptFragment) -> Result<Self, Self::Error> {
        if fragment.body().trim().is_empty() {
            return Err(crate::CoreError::Context(
                "extension prompt fragment body must not be empty".into(),
            ));
        }
        Ok(Self {
            source: InstructionSource::try_from(fragment.source())?,
            layer: match fragment.layer() {
                zeta_extension_api::PromptFragmentLayer::System => InstructionLayer::System,
                zeta_extension_api::PromptFragmentLayer::Product => InstructionLayer::Product,
                zeta_extension_api::PromptFragmentLayer::Workspace => InstructionLayer::Workspace,
                zeta_extension_api::PromptFragmentLayer::Skill => InstructionLayer::Skill,
            },
            retention: match fragment.retention() {
                zeta_extension_api::PromptFragmentRetention::Required => {
                    InstructionRetention::Required
                }
                zeta_extension_api::PromptFragmentRetention::BestEffort => {
                    InstructionRetention::BestEffort
                }
            },
            body: fragment.body().to_owned(),
        })
    }
}

/// Complete immutable input to one context-planning operation.
#[derive(Clone, Debug)]
pub(crate) struct ContextInput {
    source_thread_sequence: u64,
    current_turn_id: TurnId,
    instructions: Vec<InstructionFragment>,
    evidence: Vec<ContextEvidence>,
    items: Vec<ThreadItem>,
    checkpoints: Vec<ContextCheckpoint>,
    terminal_turns: BTreeSet<TurnId>,
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
            evidence: Vec::new(),
            items: snapshot.items.clone(),
            checkpoints: snapshot.context_checkpoints.clone(),
            terminal_turns: snapshot
                .turns
                .iter()
                .filter(|turn| {
                    matches!(
                        turn.status,
                        zeta_protocol::TurnStatus::Completed
                            | zeta_protocol::TurnStatus::Failed
                            | zeta_protocol::TurnStatus::Interrupted
                    )
                })
                .map(|turn| turn.turn_id.clone())
                .collect(),
            item_sequences: snapshot.item_sequences.clone(),
            tools,
            budget,
        }
    }

    pub(crate) fn with_evidence(mut self, evidence: Vec<ContextEvidence>) -> Self {
        self.evidence = evidence;
        self
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

    pub(crate) fn evidence(&self) -> &[ContextEvidence] {
        &self.evidence
    }

    pub(crate) fn checkpoints(&self) -> &[ContextCheckpoint] {
        &self.checkpoints
    }

    pub(crate) fn is_terminal_turn(&self, turn_id: &TurnId) -> bool {
        self.terminal_turns.contains(turn_id)
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
