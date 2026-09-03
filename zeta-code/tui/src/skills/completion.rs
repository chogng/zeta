use super::SkillChoices;
use super::skill_choices;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::chat_input_catalog_snapshot;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;

pub(crate) struct SkillCompletion {
    pub(crate) input_catalog: ChatInputCatalog,
    pub(crate) choices: SkillChoices,
}

pub(crate) fn refresh_and_build_input_catalog(
    mut client: AppServerRequestHandle,
    server_slash_commands: Vec<SlashCommandDefinition>,
    session_id: zeta_protocol::SessionId,
    plugins_enabled: bool,
) -> Result<SkillCompletion, String> {
    let catalog = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Cached,
            session_id: Some(session_id),
        })
        .map_err(|error| error.to_string())?;
    let plugins = if plugins_enabled {
        client
            .list_plugins()
            .map_err(|error| error.to_string())?
            .packages
    } else {
        Vec::new()
    };
    let input_catalog = chat_input_catalog_snapshot(&server_slash_commands, &catalog, &plugins)
        .map_err(|error| error.to_string())?;
    Ok(SkillCompletion {
        input_catalog,
        choices: skill_choices(&catalog),
    })
}
