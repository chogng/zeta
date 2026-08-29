use crate::components::composer::SkillSelectorItem;
use crate::components::composer::SlashCommandCatalog;
use crate::components::composer::built_in_slash_command_definitions;
use std::collections::BTreeMap;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::skills::SkillCompatibilityDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillListResult;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_protocol::SkillRef;

pub(crate) struct ComposerCatalogSnapshot {
    pub(crate) catalog: SlashCommandCatalog,
    pub(crate) skills: Vec<SkillSelectorItem>,
}

pub(crate) fn slash_command_registry(
    definitions: &[SlashCommandDefinition],
) -> Result<SlashCommandCatalog, ClientError> {
    SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        definitions.iter().cloned(),
    )
    .map_err(|error| {
        ClientError::Protocol(format!(
            "App Server advertised an invalid slash command snapshot: {error}"
        ))
    })
}

pub(crate) fn composer_catalog_snapshot(
    definitions: &[SlashCommandDefinition],
    skills: &SkillListResult,
) -> Result<ComposerCatalogSnapshot, ClientError> {
    let mut name_counts = BTreeMap::new();
    for skill in skills.skills.iter().filter(|skill| {
        skill.enablement == SkillEnablementDto::Enabled
            && matches!(skill.compatibility, SkillCompatibilityDto::Compatible)
    }) {
        *name_counts
            .entry(skill.id.name.as_str().to_owned())
            .or_insert(0usize) += 1;
    }
    let mut selector_items = Vec::new();
    for skill in &skills.skills {
        if skill.enablement != SkillEnablementDto::Enabled
            || !matches!(skill.compatibility, SkillCompatibilityDto::Compatible)
            || name_counts.get(skill.id.name.as_str()) != Some(&1)
        {
            continue;
        }
        let name = skill.id.name.as_str().to_owned();
        selector_items.push(SkillSelectorItem::new(
            name,
            skill.description.clone(),
            SkillRef::pinned(skill.id.clone(), skill.content_digest.clone()),
        ));
    }
    Ok(ComposerCatalogSnapshot {
        catalog: slash_command_registry(definitions)?,
        skills: selector_items,
    })
}
