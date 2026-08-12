use super::InstructionFragment;
use super::InstructionLayer;
use super::InstructionRetention;
use super::InstructionSource;
use crate::CoreError;
use zeta_protocol::ContentDigest;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillId;

/// Whether one resolved Skill instruction is required for the invocation or may yield to budget
/// pressure.
///
/// Selection policy belongs to the provider. Core only applies the declared context behavior and
/// never infers it from a picker, slash command, automatic matcher, or other activation mechanism.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillInstructionRetention {
    Required,
    BestEffort,
}

/// One exact, model-facing Skill instruction resolved by an external Skill runtime.
///
/// The value deliberately contains no catalog generation or activation reason. Those details
/// belong to discovery and selection. Core needs only stable identity, exact content revision,
/// model-facing body, and the caller's explicit budget behavior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillInstruction {
    id: SkillId,
    revision: ContentDigest,
    body: String,
    retention: SkillInstructionRetention,
}

impl SkillInstruction {
    pub fn new(
        id: SkillId,
        revision: ContentDigest,
        body: impl Into<String>,
        retention: SkillInstructionRetention,
    ) -> Self {
        Self {
            id,
            revision,
            body: body.into(),
            retention,
        }
    }

    pub fn id(&self) -> &SkillId {
        &self.id
    }

    pub fn revision(&self) -> &ContentDigest {
        &self.revision
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub const fn retention(&self) -> SkillInstructionRetention {
        self.retention
    }

    pub(crate) fn context_fragment(&self) -> InstructionFragment {
        let identity = format!("{}:{}", self.id.source, self.id.name);
        InstructionFragment::new(
            InstructionSource::new("skill", identity, self.revision.as_str()),
            InstructionLayer::Skill,
            match self.retention {
                SkillInstructionRetention::Required => InstructionRetention::Required,
                SkillInstructionRetention::BestEffort => InstructionRetention::BestEffort,
            },
            format!(
                "<skill-instructions source=\"{}\" name=\"{}\" revision=\"{}\">\n{}\n</skill-instructions>",
                escape_attribute(self.id.source.as_str()),
                escape_attribute(self.id.name.as_str()),
                escape_attribute(self.revision.as_str()),
                self.body.trim(),
            ),
        )
    }
}

/// Resolves exact Skill instructions for durable activations at each model-invocation safe point.
///
/// Implementations own Skill discovery, enablement, compatibility, source access, activation
/// policy, and exact-content loading. They must preserve request order and return one instruction
/// whose identity and revision match each durable activation. Core validates that binding before
/// admitting the instructions into context.
pub trait SkillInstructionsProvider: Send + Sync {
    fn resolve(
        &self,
        activations: &[FrozenSkillActivation],
    ) -> Result<Vec<SkillInstruction>, CoreError>;
}

pub(crate) struct NoSkillInstructions;

impl SkillInstructionsProvider for NoSkillInstructions {
    fn resolve(
        &self,
        activations: &[FrozenSkillActivation],
    ) -> Result<Vec<SkillInstruction>, CoreError> {
        if activations.is_empty() {
            Ok(Vec::new())
        } else {
            Err(CoreError::Context(
                "this runtime cannot resolve the Turn's frozen Skill activations".into(),
            ))
        }
    }
}

pub(crate) fn resolve_skill_instructions(
    provider: &dyn SkillInstructionsProvider,
    activations: &[FrozenSkillActivation],
) -> Result<Vec<SkillInstruction>, CoreError> {
    let instructions = provider.resolve(activations)?;
    if instructions.len() != activations.len() {
        return Err(CoreError::Context(format!(
            "Skill instruction provider returned {} instructions for {} durable activations",
            instructions.len(),
            activations.len(),
        )));
    }
    for (activation, instruction) in activations.iter().zip(&instructions) {
        if instruction.id() != &activation.id
            || instruction.revision() != &activation.content_digest
        {
            return Err(CoreError::Context(format!(
                "Skill instruction provider returned content that does not match durable activation '{}:{}'",
                activation.id.source, activation.id.name,
            )));
        }
        if instruction.body().trim().is_empty() {
            return Err(CoreError::Context(format!(
                "Skill instruction provider returned an empty body for '{}:{}'",
                activation.id.source, activation.id.name,
            )));
        }
    }
    Ok(instructions)
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
