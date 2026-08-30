use serde::Serialize;
use std::collections::BTreeSet;
use zeta_core::ThreadSnapshot;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolExecutionAuthority;
use zeta_protocol::ToolSourceProvenance;
use zeta_protocol::TurnId;
use zeta_work_coordination::ExternalEffectsStatus;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolEffectRecord {
    turn_id: TurnId,
    tool_call_id: zeta_protocol::ToolCallId,
    name: String,
    arguments_digest: ContentDigest,
    binding: Option<ToolCallBinding>,
    start: Option<ToolEffectStart>,
    result_digest: ContentDigest,
    outside_managed_effect_boundary: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolEffectStart {
    action_digest: String,
    policy_revision: String,
    authority: ToolExecutionAuthority,
}

pub(super) struct WorkAttemptEffectsEvidence {
    pub(super) digest: ContentDigest,
    pub(super) status: ExternalEffectsStatus,
}

/// Rebuilds the effect identity from the authoritative Thread log.
///
/// Only host-owned file and plan tools are currently proven to stay inside captured state. Every
/// other executed tool remains `Unknown` until its effect adapter supplies a verifiable receipt.
pub(super) fn work_attempt_effects(
    thread: &ThreadSnapshot,
    turn_ids: &BTreeSet<TurnId>,
) -> Result<WorkAttemptEffectsEvidence, String> {
    let mut records = Vec::new();
    let mut unknown = false;
    for item in thread.items.iter().filter(|item| {
        matches!(
            item,
            ThreadItem::ToolCall { turn_id, .. } if turn_ids.contains(turn_id)
        )
    }) {
        let ThreadItem::ToolCall {
            turn_id,
            tool_call_id,
            name,
            arguments_json,
            binding,
            ..
        } = item
        else {
            continue;
        };
        let result = thread
            .items
            .iter()
            .find(|candidate| {
                matches!(
                    candidate,
                    ThreadItem::ToolResult {
                        turn_id: result_turn_id,
                        tool_call_id: result_call_id,
                        ..
                    } if result_turn_id == turn_id && result_call_id == tool_call_id
                )
            })
            .ok_or_else(|| format!("Tool Call {tool_call_id} has no durable result"))?;
        let start = thread.tool_execution_starts.get(tool_call_id);
        let outside_managed_effect_boundary =
            has_unknown_external_effect(name.as_str(), binding.as_ref(), start.is_some());
        unknown |= outside_managed_effect_boundary;
        records.push(ToolEffectRecord {
            turn_id: turn_id.clone(),
            tool_call_id: tool_call_id.clone(),
            name: name.to_string(),
            arguments_digest: ContentDigest::sha256(arguments_json.as_bytes()),
            binding: binding.clone(),
            start: start.map(|start| ToolEffectStart {
                action_digest: start.action_digest.clone(),
                policy_revision: start.policy_revision.clone(),
                authority: start.authority.clone(),
            }),
            result_digest: ContentDigest::sha256(
                &serde_json::to_vec(result).map_err(|error| error.to_string())?,
            ),
            outside_managed_effect_boundary,
        });
    }
    let encoded = serde_json::to_vec(&(1_u32, &records)).map_err(|error| error.to_string())?;
    Ok(WorkAttemptEffectsEvidence {
        digest: ContentDigest::sha256(&encoded),
        status: if unknown {
            ExternalEffectsStatus::Unknown
        } else {
            ExternalEffectsStatus::None
        },
    })
}

fn has_unknown_external_effect(
    name: &str,
    binding: Option<&ToolCallBinding>,
    started: bool,
) -> bool {
    started && !is_confined_tool(name, binding)
}

fn is_confined_tool(name: &str, binding: Option<&ToolCallBinding>) -> bool {
    let Some(binding) = binding else {
        return false;
    };
    if !matches!(
        binding.source_chain.as_slice(),
        [ToolSourceProvenance::Product { component }] if component == "zeta-app-server"
    ) {
        return false;
    }
    matches!(
        name,
        "read_file"
            | "write_file"
            | "edit"
            | "grep"
            | "glob"
            | "apply_patch"
            | "agent_grep"
            | "update_plan"
    )
}

#[cfg(test)]
#[path = "work_attempt_effects_tests.rs"]
mod tests;
