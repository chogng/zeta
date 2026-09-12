use crate::CapabilityToolContribution;
use crate::CapabilityToolContributor;
use crate::ContextContributor;
use crate::ContextEvidence;
use crate::ContextSourceRequest;
use crate::IdleContributor;
use crate::ItemContributor;
use crate::LifecycleObserver;
use crate::PromptFragment;
use crate::ReadOnlyToolContributor;
use crate::SkillActivationContext;
use crate::SkillActivationContributor;
use crate::ThreadContext;
use crate::ThreadLifecycle;
use crate::TurnInputContext;
use crate::TurnInputContributor;
use async_utils::CancellationToken;
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
    lifecycle: Vec<Arc<dyn LifecycleObserver>>,
    idle: Vec<Arc<dyn IdleContributor>>,
    items: Vec<Arc<dyn ItemContributor>>,
    capability_tools: Vec<(&'static str, Arc<dyn CapabilityToolContributor>)>,
    read_only_tools: Vec<(&'static str, Arc<dyn ReadOnlyToolContributor>)>,
    context: Vec<(&'static str, Arc<dyn ContextContributor>)>,
    skill_activation: Vec<Arc<dyn SkillActivationContributor>>,
    turn_input: Vec<(&'static str, Arc<dyn TurnInputContributor>)>,
}

impl ExtensionRegistryBuilder {
    pub fn from_registry(registry: &ExtensionRegistry) -> Self {
        Self {
            lifecycle: registry.lifecycle.clone(),
            idle: registry.idle.clone(),
            items: registry.items.clone(),
            capability_tools: registry.capability_tools.clone(),
            read_only_tools: registry.read_only_tools.clone(),
            context: registry.context.clone(),
            skill_activation: registry.skill_activation.clone(),
            turn_input: registry.turn_input.clone(),
        }
    }
    pub fn lifecycle_observer(&mut self, observer: Arc<dyn LifecycleObserver>) -> &mut Self {
        self.lifecycle.push(observer);
        self
    }
    pub fn idle_contributor(&mut self, contributor: Arc<dyn IdleContributor>) -> &mut Self {
        self.idle.push(contributor);
        self
    }
    pub fn item_contributor(&mut self, contributor: Arc<dyn ItemContributor>) -> &mut Self {
        self.items.push(contributor);
        self
    }

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

    /// Installs or replaces one extension's read-only tools in its existing registration position.
    pub fn read_only_tool_contributor(
        &mut self,
        name: &'static str,
        contributor: Arc<dyn ReadOnlyToolContributor>,
    ) -> &mut Self {
        if let Some((_, current)) = self.read_only_tools.iter_mut().find(|(id, _)| *id == name) {
            *current = contributor;
        } else {
            self.read_only_tools.push((name, contributor));
        }
        self
    }

    /// Installs or replaces one extension's evidence source without changing registration order.
    pub fn context_contributor(
        &mut self,
        name: &'static str,
        contributor: Arc<dyn ContextContributor>,
    ) -> &mut Self {
        if let Some((_, current)) = self.context.iter_mut().find(|(id, _)| *id == name) {
            *current = contributor;
        } else {
            self.context.push((name, contributor));
        }
        self
    }

    pub fn capability_tool_contributor(
        &mut self,
        name: &'static str,
        contributor: Arc<dyn CapabilityToolContributor>,
    ) -> &mut Self {
        if let Some((_, current)) = self.capability_tools.iter_mut().find(|(id, _)| *id == name) {
            *current = contributor;
        } else {
            self.capability_tools.push((name, contributor));
        }
        self
    }

    /// Installs or replaces one extension's invocation instructions in registration order.
    pub fn turn_input_contributor(
        &mut self,
        name: &'static str,
        contributor: Arc<dyn TurnInputContributor>,
    ) -> &mut Self {
        if let Some((_, current)) = self.turn_input.iter_mut().find(|(id, _)| *id == name) {
            *current = contributor;
        } else {
            self.turn_input.push((name, contributor));
        }
        self
    }

    pub fn build(self) -> ExtensionRegistry {
        ExtensionRegistry {
            lifecycle: self.lifecycle,
            idle: self.idle,
            items: self.items,
            capability_tools: self.capability_tools,
            read_only_tools: self.read_only_tools,
            context: self.context,
            skill_activation: self.skill_activation,
            turn_input: self.turn_input,
        }
    }
}

#[derive(Default)]
pub struct ExtensionRegistry {
    lifecycle: Vec<Arc<dyn LifecycleObserver>>,
    idle: Vec<Arc<dyn IdleContributor>>,
    items: Vec<Arc<dyn ItemContributor>>,
    capability_tools: Vec<(&'static str, Arc<dyn CapabilityToolContributor>)>,
    read_only_tools: Vec<(&'static str, Arc<dyn ReadOnlyToolContributor>)>,
    context: Vec<(&'static str, Arc<dyn ContextContributor>)>,
    skill_activation: Vec<Arc<dyn SkillActivationContributor>>,
    turn_input: Vec<(&'static str, Arc<dyn TurnInputContributor>)>,
}

impl ExtensionRegistry {
    pub fn collect_context(
        &self,
        request: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, ExtensionError> {
        let mut evidence = Vec::new();
        for (_, contributor) in &self.context {
            cancellation
                .check()
                .map_err(|signal| ExtensionError::new(signal.reason().to_string()))?;
            evidence.extend(contributor.collect(request, cancellation)?);
        }
        cancellation
            .check()
            .map_err(|signal| ExtensionError::new(signal.reason().to_string()))?;
        Ok(evidence)
    }

    pub fn thread_changed(&self, context: ThreadContext<'_>, event: &ThreadLifecycle) {
        for observer in &self.lifecycle {
            observer.thread_changed(context, event);
        }
        if matches!(
            event,
            ThreadLifecycle::TurnCompleted(_)
                | ThreadLifecycle::TurnFailed(_)
                | ThreadLifecycle::TurnInterrupted(_)
        ) {
            for contributor in &self.idle {
                contributor.contribute(context);
            }
        }
    }
    pub fn config_changed(&self, generation: u64) {
        for observer in &self.lifecycle {
            observer.config_changed(generation);
        }
    }
    pub fn contribute_items(
        &self,
        context: ThreadContext<'_>,
    ) -> Result<Vec<extension_items::ExtensionItem>, ExtensionError> {
        let mut items = Vec::new();
        let mut identities = BTreeSet::new();
        for contributor in &self.items {
            let contributed = contributor.contribute(context)?;
            if items.len() + contributed.len() > 128 {
                return Err(ExtensionError::new("too many extension items"));
            }
            for item in contributed {
                item.validate().map_err(ExtensionError::new)?;
                if !identities.insert((item.extension.clone(), item.id.clone())) {
                    return Err(ExtensionError::new("duplicate extension item identity"));
                }
                items.push(item);
            }
        }
        Ok(items)
    }

    pub fn contribute_capability_tools(
        &self,
    ) -> Result<Vec<CapabilityToolContribution>, ExtensionError> {
        let mut tools = Vec::new();
        let mut names = BTreeSet::new();
        for (_, contributor) in &self.capability_tools {
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
        for (_, contributor) in &self.read_only_tools {
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
        for (_, contributor) in &self.turn_input {
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
