use crate::ThreadSnapshot;
use crate::context::InstructionFragment;
use crate::context::InstructionLayer;
use crate::context::InstructionRetention;
use crate::context::InstructionSource;
use std::collections::BTreeSet;
use zeta_protocol::AgentContextContent;
use zeta_protocol::AgentMessageContent;
use zeta_protocol::ToolDefinition;

pub(crate) fn agent_context_fragments(snapshot: &ThreadSnapshot) -> Vec<InstructionFragment> {
    let mut fragments = Vec::new();
    if let Some(seed) = &snapshot.agent_context_seed {
        fragments.push(InstructionFragment::new(
            InstructionSource::new("agent-role", seed.role.name.clone(), seed.digest.as_str()),
            InstructionLayer::Product,
            InstructionRetention::Required,
            format!(
                "<agent-role name=\"{}\" delegation=\"{}\">\n{}\n</agent-role>",
                xml_escape(&seed.role.name),
                seed.delegation_id,
                seed.role.instructions.trim()
            ),
        ));
        fragments.extend(seed.materialized_context.iter().map(|materialized| {
            let (source_thread_id, source_sequence, source_kind, source_id) =
                materialized_source_identity(&materialized.source);
            InstructionFragment::new(
                InstructionSource::new(
                    source_kind,
                    format!("{source_thread_id}:{source_id}"),
                    format!("{source_sequence}:{}", materialized.content_digest.as_str()),
                ),
                InstructionLayer::Workspace,
                InstructionRetention::Required,
                format!(
                    "<inherited-agent-context source-thread=\"{}\" source-sequence=\"{}\" kind=\"{}\">\n{}\n</inherited-agent-context>",
                    source_thread_id,
                    source_sequence,
                    source_kind,
                    xml_escape(&materialized_content_text(&materialized.content))
                ),
            )
        }));
    }
    let mut messages = snapshot
        .received_agent_messages
        .values()
        .collect::<Vec<_>>();
    messages.sort_by(|left, right| {
        left.sender_sequence
            .cmp(&right.sender_sequence)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    fragments.extend(messages.into_iter().map(|message| {
        let body = match &message.content {
            AgentMessageContent::Instruction { text } => text.clone(),
            AgentMessageContent::Result { result } => format!(
                "Delegation {} completed with status {:?}.\n{}",
                result.delegation_id,
                result.status,
                result.summary.trim()
            ),
        };
        InstructionFragment::new(
            InstructionSource::new(
                "agent-message",
                message.message_id.to_string(),
                message.sender_sequence.to_string(),
            ),
            InstructionLayer::Workspace,
            InstructionRetention::BestEffort,
            format!(
                "<agent-message sender=\"{}\" provenance=\"{:?}\">\n{}\n</agent-message>",
                message.sender_thread_id,
                message.provenance,
                xml_escape(&body)
            ),
        )
    }));
    fragments
}

fn materialized_source_identity(
    source: &zeta_protocol::AgentContextSource,
) -> (&zeta_protocol::ThreadId, u64, &'static str, String) {
    match source {
        zeta_protocol::AgentContextSource::Item {
            source_thread_id,
            source_sequence,
            item_id,
        } => (
            source_thread_id,
            *source_sequence,
            "item",
            item_id.to_string(),
        ),
        zeta_protocol::AgentContextSource::Checkpoint {
            source_thread_id,
            source_sequence,
            checkpoint_id,
        } => (
            source_thread_id,
            *source_sequence,
            "checkpoint",
            checkpoint_id.to_string(),
        ),
    }
}

fn materialized_content_text(content: &AgentContextContent) -> String {
    match content {
        AgentContextContent::UserText { text } => format!("User: {text}"),
        AgentContextContent::UserImage { url } => format!("User image: {url}"),
        AgentContextContent::AssistantText { text } => format!("Assistant: {text}"),
        AgentContextContent::Reasoning { text } => format!("Reasoning record: {text}"),
        AgentContextContent::Plan { text } => format!("Plan record: {text}"),
        AgentContextContent::ToolCall {
            name,
            arguments_json,
        } => format!("Tool call {name}: {arguments_json}"),
        AgentContextContent::ToolResult { text, is_error } => {
            format!("Tool result (error={is_error}): {text}")
        }
        AgentContextContent::Checkpoint { summary } => format!("Checkpoint: {summary}"),
    }
}

pub(crate) fn scope_agent_tools(
    snapshot: &ThreadSnapshot,
    tools: Vec<ToolDefinition>,
) -> Vec<ToolDefinition> {
    let Some(seed) = &snapshot.agent_context_seed else {
        return tools;
    };
    let allowed = seed
        .capability_scope
        .tools
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    tools
        .into_iter()
        .filter(|tool| allowed.contains(&tool.name))
        .collect()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
