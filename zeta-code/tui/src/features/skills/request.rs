use super::SkillPaneSpec;
use super::skills_pane_spec;
use crate::client::new_command_id;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;
use zeta_app_server_protocol::protocol::skills::SkillSetEnablementParams;
use zeta_protocol::{SessionId, SkillId};

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    reload: SkillCatalogReloadDto,
) -> Result<SkillPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_skills(SkillListParams {
            reload,
            session_id: Some(session_id.clone()),
        })
        .map(|catalog| skills_pane_spec(&catalog))
}

pub(crate) fn set_enablement<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    skill_id: SkillId,
    enablement: SkillEnablementDto,
) -> Result<SkillPaneSpec, ClientError>
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
    load_selection(client, session_id, SkillCatalogReloadDto::Cached)
}
