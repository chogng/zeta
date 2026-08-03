use super::SkillSelectionView;
use super::skills_selection_view;
use crate::client::new_command_id;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;
use zeta_app_server_protocol::protocol::skills::SkillSetEnablementParams;
use zeta_protocol::SkillId;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    reload: SkillCatalogReloadDto,
) -> Result<SkillSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_skills(SkillListParams { reload })
        .map(|catalog| skills_selection_view(&catalog))
}

pub(crate) fn set_enablement<T>(
    client: &mut AppServerClient<T>,
    skill_id: SkillId,
    enablement: SkillEnablementDto,
) -> Result<SkillSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    client.set_skill_enablement(SkillSetEnablementParams {
        command_id: new_command_id("skill-enablement"),
        expected_revision: config.revision,
        skill_id,
        enablement,
    })?;
    load_selection(client, SkillCatalogReloadDto::Cached)
}
