use crate::CapabilityToolContribution;
use crate::CapabilityToolContributor;
use crate::ExtensionError;
use crate::ExtensionRegistryBuilder;
use crate::ExtensionToolAuthority;
use crate::PromptFragment;
use crate::ReadOnlyToolContributor;
use crate::SkillActivationContext;
use crate::SkillActivationContributor;
use std::sync::Arc;
use zeta_protocol::ContentDigest;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillSourceId;
use zeta_tools::ToolDefinition;
use zeta_tools::ToolExecutionFuture;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolExecutor;
use zeta_tools::ToolInputSchema;
use zeta_tools::ToolInvocation;
use zeta_tools::ToolLoading;
use zeta_tools::ToolName;
use zeta_tools::ToolOutput;
use zeta_tools::ToolOutputSchema;
use zeta_tools::ToolSchemaMode;

struct FixedActivationContributor {
    activation: FrozenSkillActivation,
}

impl SkillActivationContributor for FixedActivationContributor {
    fn contribute(
        &self,
        _: SkillActivationContext<'_>,
    ) -> Result<Vec<FrozenSkillActivation>, ExtensionError> {
        Ok(vec![self.activation.clone()])
    }
}

#[test]
fn duplicate_skill_activations_from_extensions_are_rejected() {
    let activation = activation("review");
    let mut builder = ExtensionRegistryBuilder::new();
    builder.skill_activation_contributor(Arc::new(FixedActivationContributor {
        activation: activation.clone(),
    }));
    builder.skill_activation_contributor(Arc::new(FixedActivationContributor { activation }));
    let registry = builder.build();

    let error = registry
        .contribute_skill_activations(SkillActivationContext::new(&[]))
        .unwrap_err();

    assert!(error.to_string().contains("multiple extensions activated"));
}

struct FixedToolContributor;

impl ReadOnlyToolContributor for FixedToolContributor {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        Ok(vec![Arc::new(FixedExecutor)])
    }
}

struct FixedExecutor;

impl ToolExecutor for FixedExecutor {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function(
            ToolName::new("read-extension-state").unwrap(),
            "Read immutable extension state.",
            ToolInputSchema::parse(serde_json::json!({
                "type": "object",
                "properties": {}
            }))
            .unwrap(),
            ToolOutputSchema::Unspecified,
            ToolSchemaMode::Strict,
            ToolLoading::Eager,
        )
        .unwrap()
    }

    fn execute(&self, _: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(std::future::ready(ToolExecutionOutcome::Returned(
            ToolOutput::success(Vec::new()),
        )))
    }
}

#[test]
fn duplicate_read_only_tool_names_from_extensions_are_rejected() {
    let mut builder = ExtensionRegistryBuilder::new();
    builder.read_only_tool_contributor(Arc::new(FixedToolContributor));
    builder.read_only_tool_contributor(Arc::new(FixedToolContributor));

    let error = match builder.build().contribute_read_only_tools() {
        Ok(_) => panic!("duplicate extension tool names were accepted"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("multiple extensions contributed")
    );
}

struct FixedCapabilityContributor;

impl CapabilityToolContributor for FixedCapabilityContributor {
    fn contribute(&self) -> Result<Vec<CapabilityToolContribution>, ExtensionError> {
        Ok(vec![CapabilityToolContribution::new(
            Arc::new(FixedExecutor),
            ExtensionToolAuthority::ExternalRead {
                service: "search".into(),
                network_scopes: vec!["search.example.com".into()],
                credential_reference: None,
            },
        )])
    }
}

#[test]
fn capability_tool_contributions_preserve_declared_authority() {
    let mut builder = ExtensionRegistryBuilder::new();
    builder.capability_tool_contributor(Arc::new(FixedCapabilityContributor));

    let tools = builder.build().contribute_capability_tools().unwrap();

    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0].authority(),
        &ExtensionToolAuthority::ExternalRead {
            service: "search".into(),
            network_scopes: vec!["search.example.com".into()],
            credential_reference: None,
        }
    );
}

fn activation(name: &str) -> FrozenSkillActivation {
    FrozenSkillActivation {
        id: SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new(name).unwrap(),
        ),
        content_digest: ContentDigest::sha256(name.as_bytes()),
        catalog_generation: 1,
        reason: SkillActivationReason::Explicit,
    }
}

#[allow(dead_code)]
fn assert_prompt_fragment_is_public(_: PromptFragment) {}
