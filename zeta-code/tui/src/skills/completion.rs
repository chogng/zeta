use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_protocol::protocol::plugins::PluginPackageDto;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;
use zeta_app_server_protocol::protocol::skills::SkillListResult;

pub(crate) struct SkillRefresh {
    pub(crate) catalog: SkillListResult,
    pub(crate) plugins: Vec<PluginPackageDto>,
}

pub(crate) fn refresh(
    mut client: AppServerRequestHandle,
    session_id: zeta_protocol::SessionId,
    plugins_enabled: bool,
) -> Result<SkillRefresh, String> {
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
    Ok(SkillRefresh { catalog, plugins })
}
