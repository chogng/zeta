use super::*;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillName;
use zeta_protocol::SkillSourceId;

#[test]
fn resolved_skill_context_contains_only_canonical_injection_metadata() {
    let instruction = instruction(
        "review",
        SkillInstructionRetention::Required,
        "Review safely.",
    );

    let fragment = instruction.context_fragment();

    assert_eq!(fragment.layer(), InstructionLayer::Skill);
    assert_eq!(fragment.retention(), InstructionRetention::Required);
    assert_eq!(fragment.source().kind(), "skill");
    assert_eq!(
        fragment.source().identity(),
        "user:skill-source:test:review"
    );
    assert_eq!(
        fragment.source().revision(),
        instruction.revision().as_str()
    );
    assert!(fragment.body().contains("name=\"review\""));
    assert!(fragment.body().contains(instruction.revision().as_str()));
    assert!(fragment.body().contains("Review safely."));
    assert!(!fragment.body().contains("catalog-generation"));
    assert!(!fragment.body().contains("reason="));
}

#[test]
fn provider_output_count_must_match_durable_activations() {
    let provider = FixedProvider {
        instructions: Vec::new(),
    };

    let error =
        resolve_skill_instructions(&provider, &[activation("review", b"review")]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("0 instructions for 1 durable activations")
    );
}

#[test]
fn provider_output_must_preserve_durable_identity_revision_and_order() {
    let first = activation("first", b"first");
    let second = activation("second", b"second");
    let provider = FixedProvider {
        instructions: vec![
            instruction("second", SkillInstructionRetention::Required, "second"),
            instruction("first", SkillInstructionRetention::Required, "first"),
        ],
    };

    let error = resolve_skill_instructions(&provider, &[first, second]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("does not match durable activation")
    );
}

#[test]
fn best_effort_policy_maps_without_knowing_activation_reason() {
    let instruction = instruction(
        "review",
        SkillInstructionRetention::BestEffort,
        "Review safely.",
    );

    assert_eq!(
        instruction.context_fragment().retention(),
        InstructionRetention::BestEffort
    );
}

struct FixedProvider {
    instructions: Vec<SkillInstruction>,
}

impl SkillInstructionsProvider for FixedProvider {
    fn resolve(&self, _: &[FrozenSkillActivation]) -> Result<Vec<SkillInstruction>, CoreError> {
        Ok(self.instructions.clone())
    }
}

fn activation(name: &str, body: &[u8]) -> FrozenSkillActivation {
    FrozenSkillActivation {
        id: id(name),
        content_digest: ContentDigest::sha256(body),
        catalog_generation: 42,
        reason: SkillActivationReason::Automatic,
    }
}

fn instruction(name: &str, retention: SkillInstructionRetention, body: &str) -> SkillInstruction {
    SkillInstruction::new(
        id(name),
        ContentDigest::sha256(name.as_bytes()),
        body,
        retention,
    )
}

fn id(name: &str) -> SkillId {
    SkillId::new(
        SkillSourceId::new("user:skill-source:test").unwrap(),
        SkillName::new(name).unwrap(),
    )
}
