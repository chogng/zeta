use std::collections::BTreeSet;

use zeta_agents::AgentDefinition;
use zeta_agents::AgentDefinitionCatalogSnapshot;
use zeta_core::CoreError;
use zeta_instructions::InstructionCatalogSnapshot;
use zeta_protocol::AgentDefinitionSelectionReason;
use zeta_protocol::AgentRoleSnapshot;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegatedCapabilityScope;
use zeta_protocol::FrozenAgentDefinitionRef;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::ToolName;

const MIN_AUTOMATIC_SCORE: u64 = 60;

pub(super) struct ResolvedAgentSelection {
    pub(super) role: AgentRoleSnapshot,
    pub(super) capability_scope: DelegatedCapabilityScope,
}

pub(super) fn resolve_agent_selection(
    requested: Option<&str>,
    task: &str,
    current_model: Option<&ModelRef>,
    available_tools: Vec<ToolName>,
    active_skills: &[FrozenSkillActivation],
    agents: &[std::sync::Arc<AgentDefinitionCatalogSnapshot>],
    instructions: &[std::sync::Arc<InstructionCatalogSnapshot>],
) -> Result<ResolvedAgentSelection, CoreError> {
    let selected = match requested {
        Some(name) => {
            let matches = agents
                .iter()
                .flat_map(|snapshot| {
                    snapshot
                        .entries()
                        .iter()
                        .filter(move |definition| definition.name() == name)
                        .map(|definition| (definition, snapshot.generation()))
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [(definition, generation)] => Some((
                    *definition,
                    AgentDefinitionSelectionReason::Explicit,
                    *generation,
                )),
                [] => {
                    return Err(CoreError::InvalidInput(format!(
                        "Workspace Agent definition '{name}' is not available"
                    )));
                }
                _ => {
                    return Err(CoreError::InvalidInput(format!(
                        "Workspace Agent definition '{name}' is ambiguous across authorized directories"
                    )));
                }
            }
        }
        None => select_automatic(agents, task).map(|(definition, generation)| {
            (
                definition,
                AgentDefinitionSelectionReason::Automatic,
                generation,
            )
        }),
    };
    let Some((definition, selection_reason, catalog_generation)) = selected else {
        return Ok(general_selection(
            current_model,
            available_tools,
            active_skills,
        ));
    };
    let tools = resolve_tools(definition, available_tools)?;
    let skills = resolve_skills(definition, active_skills)?;
    let role_instructions = resolve_role_instructions(definition, instructions)?;
    let model = definition
        .model()
        .map(parse_model_ref)
        .transpose()?
        .or_else(|| current_model.cloned());
    let content_digest = ContentDigest::new(definition.content_digest())
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let frozen = FrozenAgentDefinitionRef {
        name: definition.name().into(),
        catalog_generation,
        content_digest,
        selection_reason,
    };
    Ok(ResolvedAgentSelection {
        role: AgentRoleSnapshot {
            name: definition.name().into(),
            instructions: role_instructions,
            model,
            definition: Some(frozen),
        },
        capability_scope: DelegatedCapabilityScope { tools, skills },
    })
}

fn general_selection(
    current_model: Option<&ModelRef>,
    available_tools: Vec<ToolName>,
    active_skills: &[FrozenSkillActivation],
) -> ResolvedAgentSelection {
    ResolvedAgentSelection {
        role: AgentRoleSnapshot {
            name: "general".into(),
            instructions: "Complete the delegated task independently. Return a concise, evidence-backed result to the parent Agent.".into(),
            model: current_model.cloned(),
            definition: None,
        },
        capability_scope: DelegatedCapabilityScope {
            tools: available_tools,
            skills: active_skills
                .iter()
                .cloned()
                .map(as_automatic)
                .collect(),
        },
    }
}

fn resolve_tools(
    definition: &AgentDefinition,
    available_tools: Vec<ToolName>,
) -> Result<Vec<ToolName>, CoreError> {
    let available = available_tools.into_iter().collect::<BTreeSet<_>>();
    definition
        .tools()
        .iter()
        .map(|reference| {
            let name = ToolName::new(reference.clone())
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            available.contains(&name).then_some(name).ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "Agent definition '{}' requires unavailable tool '{reference}'",
                    definition.name()
                ))
            })
        })
        .collect()
}

fn resolve_skills(
    definition: &AgentDefinition,
    active: &[FrozenSkillActivation],
) -> Result<Vec<FrozenSkillActivation>, CoreError> {
    definition
        .skills()
        .iter()
        .map(|reference| {
            let matches = active
                .iter()
                .filter(|activation| skill_matches(reference, activation))
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [matched] => Ok(as_automatic((*matched).clone())),
                [] => Err(CoreError::InvalidInput(format!(
                    "Agent definition '{}' requires inactive Skill '{reference}'",
                    definition.name()
                ))),
                _ => Err(CoreError::InvalidInput(format!(
                    "Agent definition '{}' has ambiguous Skill reference '{reference}'",
                    definition.name()
                ))),
            }
        })
        .collect()
}

fn skill_matches(reference: &str, activation: &FrozenSkillActivation) -> bool {
    reference == activation.id.name.as_str()
        || reference
            == format!(
                "{}/{}",
                activation.id.source.as_str(),
                activation.id.name.as_str()
            )
}

fn as_automatic(mut activation: FrozenSkillActivation) -> FrozenSkillActivation {
    activation.reason = SkillActivationReason::Automatic;
    activation
}

fn resolve_role_instructions(
    definition: &AgentDefinition,
    instructions: &[std::sync::Arc<InstructionCatalogSnapshot>],
) -> Result<String, CoreError> {
    let mut body = definition.role_instructions().to_owned();
    for reference in definition.instructions() {
        let matches = instructions
            .iter()
            .flat_map(|snapshot| {
                snapshot
                    .entries()
                    .iter()
                    .filter(|artifact| artifact.name() == reference)
            })
            .collect::<Vec<_>>();
        let artifact = match matches.as_slice() {
            [artifact] => *artifact,
            [] => {
                return Err(CoreError::InvalidInput(format!(
                    "Agent definition '{}' requires unavailable Instruction '{reference}'",
                    definition.name()
                )));
            }
            _ => {
                return Err(CoreError::InvalidInput(format!(
                    "Agent definition '{}' has ambiguous Instruction reference '{reference}'",
                    definition.name()
                )));
            }
        };
        body.push_str(&format!(
            "\n\n<agent-instruction name=\"{}\">\n{}\n</agent-instruction>",
            artifact.name(),
            artifact.body()
        ));
    }
    Ok(body)
}

fn parse_model_ref(reference: &str) -> Result<ModelRef, CoreError> {
    let (provider, model) = reference.split_once('/').ok_or_else(|| {
        CoreError::InvalidInput(format!(
            "Agent model reference '{reference}' must use provider/model"
        ))
    })?;
    Ok(ModelRef::new(
        ProviderId::new(provider).map_err(|error| CoreError::InvalidInput(error.to_string()))?,
        ModelId::new(model).map_err(|error| CoreError::InvalidInput(error.to_string()))?,
    ))
}

fn select_automatic<'a>(
    snapshots: &'a [std::sync::Arc<AgentDefinitionCatalogSnapshot>],
    task: &str,
) -> Option<(&'a AgentDefinition, u64)> {
    let task = normalize(task);
    let task_tokens = tokens(&task);
    let mut candidates = snapshots
        .iter()
        .flat_map(|snapshot| {
            snapshot.entries().iter().filter_map(|definition| {
                let score = selection_score(definition, &task, &task_tokens);
                (score >= MIN_AUTOMATIC_SCORE).then_some((score, definition, snapshot.generation()))
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left_score, left, _), (right_score, right, _)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.name().cmp(right.name()))
    });
    let (best_score, best, generation) = candidates.first()?;
    if candidates
        .get(1)
        .is_some_and(|(runner_up, _, _)| runner_up == best_score)
    {
        return None;
    }
    Some((best, *generation))
}

fn selection_score(
    definition: &AgentDefinition,
    task: &str,
    task_tokens: &BTreeSet<String>,
) -> u64 {
    let name_phrase = normalize(&definition.name().replace('-', " "));
    let name_tokens = tokens(&name_phrase);
    let description_tokens = tokens(definition.description());
    let name_matches = name_tokens.intersection(task_tokens).count() as u64;
    let description_matches = description_tokens.intersection(task_tokens).count() as u64;
    let exact_name = format!(" {task} ").contains(&format!(" {name_phrase} "));
    let complete_name = name_tokens.len() >= 2 && name_matches == name_tokens.len() as u64;
    u64::from(exact_name) * 200
        + u64::from(complete_name) * 100
        + name_matches * 30
        + description_matches * 20
}

fn tokens(value: &str) -> BTreeSet<String> {
    normalize(value)
        .split_whitespace()
        .filter_map(canonical_token)
        .collect()
}

fn normalize(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut separated = true;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            separated = false;
        } else if !separated {
            normalized.push(' ');
            separated = true;
        }
    }
    normalized.trim().to_owned()
}

fn canonical_token(value: &str) -> Option<String> {
    const STOP_WORDS: &[&str] = &[
        "and", "are", "for", "from", "into", "that", "the", "this", "use", "user", "when", "with",
        "your",
    ];
    if value.len() < 3 || STOP_WORDS.contains(&value) {
        return None;
    }
    let mut token = value.to_owned();
    if token.len() > 4 && token.ends_with('s') && !token.ends_with("ss") {
        token.pop();
    }
    Some(token)
}

#[cfg(test)]
#[path = "agent_selection_tests.rs"]
mod tests;
