use crate::CapabilityToolContribution;
use crate::CapabilityToolContributor;
use crate::PromptFragment;
use crate::ReadOnlyToolContributor;
use crate::SkillActivationContext;
use crate::SkillActivationContributor;
use crate::TurnInputContext;
use crate::TurnInputContributor;
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;
use zeta_protocol::FrozenSkillActivation;
use zeta_tools::ToolExecutor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionError(String);

impl ExtensionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExtensionError {}

#[derive(Default)]
pub struct ExtensionRegistryBuilder {
    capability_tools: Vec<Arc<dyn CapabilityToolContributor>>,
    read_only_tools: Vec<Arc<dyn ReadOnlyToolContributor>>,
    skill_activation: Vec<Arc<dyn SkillActivationContributor>>,
    turn_input: Vec<Arc<dyn TurnInputContributor>>,
}

impl ExtensionRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn skill_activation_contributor(
        &mut self,
        contributor: Arc<dyn SkillActivationContributor>,
    ) -> &mut Self {
        self.skill_activation.push(contributor);
        self
    }

    pub fn read_only_tool_contributor(
        &mut self,
        contributor: Arc<dyn ReadOnlyToolContributor>,
    ) -> &mut Self {
        self.read_only_tools.push(contributor);
        self
    }

    pub fn capability_tool_contributor(
        &mut self,
        contributor: Arc<dyn CapabilityToolContributor>,
    ) -> &mut Self {
        self.capability_tools.push(contributor);
        self
    }

    pub fn turn_input_contributor(
        &mut self,
        contributor: Arc<dyn TurnInputContributor>,
    ) -> &mut Self {
        self.turn_input.push(contributor);
        self
    }

    pub fn build(self) -> ExtensionRegistry {
        ExtensionRegistry {
            capability_tools: self.capability_tools,
            read_only_tools: self.read_only_tools,
            skill_activation: self.skill_activation,
            turn_input: self.turn_input,
        }
    }
}

#[derive(Default)]
pub struct ExtensionRegistry {
    capability_tools: Vec<Arc<dyn CapabilityToolContributor>>,
    read_only_tools: Vec<Arc<dyn ReadOnlyToolContributor>>,
    skill_activation: Vec<Arc<dyn SkillActivationContributor>>,
    turn_input: Vec<Arc<dyn TurnInputContributor>>,
}

impl ExtensionRegistry {
    pub fn contribute_capability_tools(
        &self,
    ) -> Result<Vec<CapabilityToolContribution>, ExtensionError> {
        let mut tools = Vec::new();
        let mut names = BTreeSet::new();
        for contributor in &self.capability_tools {
            for contribution in contributor.contribute()? {
                let definition = contribution.executor().definition();
                if !names.insert(definition.name().clone()) {
                    return Err(ExtensionError::new(format!(
                        "multiple extensions contributed capability tool '{}'",
                        definition.name()
                    )));
                }
                tools.push(contribution);
            }
        }
        Ok(tools)
    }

    pub fn contribute_read_only_tools(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        let mut tools = Vec::new();
        let mut names = BTreeSet::new();
        for contributor in &self.read_only_tools {
            for executor in contributor.contribute()? {
                let definition = executor.definition();
                if !names.insert(definition.name().clone()) {
                    return Err(ExtensionError::new(format!(
                        "multiple extensions contributed read-only tool '{}'",
                        definition.name()
                    )));
                }
                tools.push(executor);
            }
        }
        Ok(tools)
    }

    pub fn contribute_skill_activations(
        &self,
        input: SkillActivationContext<'_>,
    ) -> Result<Vec<FrozenSkillActivation>, ExtensionError> {
        let mut activations = Vec::new();
        let mut identities = BTreeSet::new();
        for contributor in &self.skill_activation {
            for activation in contributor.contribute(match input.session_id() {
                Some(session_id) => {
                    SkillActivationContext::for_session(session_id, input.user_input())
                }
                None => SkillActivationContext::new(input.user_input()),
            })? {
                if !identities.insert(activation.id.clone()) {
                    return Err(ExtensionError::new(format!(
                        "multiple extensions activated Skill '{}:{}'",
                        activation.id.source, activation.id.name
                    )));
                }
                activations.push(activation);
            }
        }
        Ok(activations)
    }

    pub fn contribute_turn_input(
        &self,
        input: TurnInputContext<'_>,
    ) -> Result<Vec<PromptFragment>, ExtensionError> {
        let mut fragments = Vec::new();
        for contributor in &self.turn_input {
            fragments.extend(contributor.contribute(match input.session_id() {
                Some(session_id) => TurnInputContext::for_session(
                    session_id,
                    input.thread_id(),
                    input.turn_id(),
                    input.activated_skills(),
                ),
                None => TurnInputContext::new(
                    input.thread_id(),
                    input.turn_id(),
                    input.activated_skills(),
                ),
            })?);
        }
        Ok(fragments)
    }
}
