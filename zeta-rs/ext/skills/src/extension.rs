use crate::SkillRuntime;
use crate::catalog_prompt::catalog_prompt;
use crate::tool::SkillToolContributor;
use std::sync::Arc;
use zeta_extension_api::ExtensionError;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_extension_api::PromptFragment;
use zeta_extension_api::PromptFragmentLayer;
use zeta_extension_api::PromptFragmentRetention;
use zeta_extension_api::PromptFragmentSource;
use zeta_extension_api::SkillActivationContext;
use zeta_extension_api::SkillActivationContributor;
use zeta_extension_api::TurnInputContext;
use zeta_extension_api::TurnInputContributor;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::UserInput;

/// Installs one Skill runtime into the generic agent extension registry.
pub fn install(builder: &mut ExtensionRegistryBuilder, runtime: Arc<SkillRuntime>) {
    builder.skill_activation_contributor(runtime.clone());
    builder.read_only_tool_contributor(Arc::new(SkillToolContributor::new(runtime.clone())));
    builder.turn_input_contributor(runtime);
}

impl SkillActivationContributor for SkillRuntime {
    fn contribute(
        &self,
        input: SkillActivationContext<'_>,
    ) -> Result<Vec<FrozenSkillActivation>, ExtensionError> {
        let mut activations = input
            .user_input()
            .iter()
            .filter_map(|item| match item {
                UserInput::Skill { skill } => Some(skill),
                UserInput::Text { .. }
                | UserInput::Context { .. }
                | UserInput::ImageAttachment { .. }
                | UserInput::Image { .. }
                | UserInput::LocalImage { .. }
                | UserInput::Mention { .. } => None,
            })
            .map(|selected| {
                self.activate_explicit(selected)
                    .map(|skill| skill.activation().clone())
                    .map_err(ExtensionError::new)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let excluded = activations
            .iter()
            .map(|activation| activation.id.clone())
            .collect::<Vec<_>>();
        if let Some(selected) = self
            .select_automatic(input.user_input(), &excluded)
            .map_err(ExtensionError::new)?
        {
            activations.push(selected.activation().clone());
        }
        Ok(activations)
    }
}

impl TurnInputContributor for SkillRuntime {
    fn contribute(
        &self,
        input: TurnInputContext<'_>,
    ) -> Result<Vec<PromptFragment>, ExtensionError> {
        let snapshot = self
            .list(crate::SkillCatalogReload::Cached)
            .map_err(ExtensionError::new)?;
        let mut fragments = catalog_prompt(snapshot.as_ref())
            .into_iter()
            .collect::<Vec<_>>();
        let activated = input
            .activated_skills()
            .iter()
            .map(|frozen| {
                let activated = self.load_frozen(frozen).map_err(ExtensionError::new)?;
                let identity = format!("{}:{}", frozen.id.source, frozen.id.name);
                Ok(PromptFragment::new(
                    PromptFragmentSource::new(
                        "skill",
                        identity,
                        frozen.content_digest.as_str(),
                    ),
                    PromptFragmentLayer::Skill,
                    match frozen.reason {
                        SkillActivationReason::Explicit => PromptFragmentRetention::Required,
                        SkillActivationReason::Automatic => PromptFragmentRetention::BestEffort,
                    },
                    format!(
                        "<skill-instructions source=\"{}\" name=\"{}\" revision=\"{}\">\n{}\n</skill-instructions>",
                        escape_attribute(frozen.id.source.as_str()),
                        escape_attribute(frozen.id.name.as_str()),
                        escape_attribute(frozen.content_digest.as_str()),
                        activated.body().trim(),
                    ),
                ))
            })
            .collect::<Result<Vec<_>, ExtensionError>>()?;
        fragments.extend(activated);
        Ok(fragments)
    }
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
