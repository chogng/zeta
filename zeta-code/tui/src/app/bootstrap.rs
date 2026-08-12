use crate::components::composer::SlashCommandCatalog;
use crate::components::composer::built_in_slash_command_definitions;
use std::collections::BTreeMap;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::skills::SkillCompatibilityDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillListResult;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_protocol::SkillRef;

pub(crate) struct TuiSlashCommandRegistry {
    pub(crate) catalog: SlashCommandCatalog,
    pub(crate) skills: BTreeMap<String, SkillRef>,
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

pub(crate) fn skill_slash_command_registry(
    definitions: &[SlashCommandDefinition],
    skills: &SkillListResult,
) -> Result<TuiSlashCommandRegistry, ClientError> {
    let reserved_names = built_in_slash_command_definitions()
        .into_iter()
        .chain(definitions.iter().cloned())
        .map(|definition| definition.name)
        .collect::<std::collections::BTreeSet<_>>();
    let mut name_counts = BTreeMap::new();
    for skill in skills.skills.iter().filter(|skill| {
        skill.enablement == SkillEnablementDto::Enabled
            && matches!(skill.compatibility, SkillCompatibilityDto::Compatible)
    }) {
        *name_counts
            .entry(skill.id.name.as_str().to_owned())
            .or_insert(0usize) += 1;
    }
    let mut bindings = BTreeMap::new();
    let mut skill_definitions = Vec::new();
    for skill in &skills.skills {
        if skill.enablement != SkillEnablementDto::Enabled
            || !matches!(skill.compatibility, SkillCompatibilityDto::Compatible)
            || name_counts.get(skill.id.name.as_str()) != Some(&1)
            || reserved_names.contains(skill.id.name.as_str())
        {
            continue;
        }
        let name = skill.id.name.as_str().to_owned();
        bindings.insert(
            name.clone(),
            SkillRef::pinned(skill.id.clone(), skill.content_digest.clone()),
        );
        skill_definitions.push(SlashCommandDefinition {
            name,
            description: skill.description.clone(),
            argument_mode:
                zeta_app_server_protocol::protocol::slash_commands::SlashCommandArgumentModeDto::Optional,
        });
    }
    let catalog = SlashCommandCatalog::with_local_server_and_skills(
        built_in_slash_command_definitions(),
        definitions.iter().cloned(),
        skill_definitions,
    )
    .map_err(|error| {
        ClientError::Protocol(format!(
            "App Server advertised an invalid slash command or Skill snapshot: {error}"
        ))
    })?;
    Ok(TuiSlashCommandRegistry {
        catalog,
        skills: bindings,
    })
}
