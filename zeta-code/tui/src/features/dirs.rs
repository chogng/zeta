use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use std::collections::BTreeMap;
use std::path::PathBuf;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirAddParams;
use zeta_app_server_protocol::protocol::environment::SessionDirListParams;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_app_server_protocol::protocol::environment::SessionDirMutationResult;
use zeta_app_server_protocol::protocol::environment::SessionDirRemoveParams;
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DirSelectionAction {
    Remove { path: PathBuf },
}

pub(crate) struct DirPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, DirSelectionAction>,
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
) -> Result<DirPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_session_dirs(SessionDirListParams {
            session_id: session_id.clone(),
        })
        .map(pane_spec)
}

pub(crate) fn add<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    path: PathBuf,
    permissions: Vec<PermissionDto>,
) -> Result<SessionDirMutationResult, ClientError>
where
    T: JsonRpcTransport,
{
    client.add_session_dir(SessionDirAddParams {
        session_id: session_id.clone(),
        path,
        permissions,
    })
}

pub(crate) fn remove<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    path: PathBuf,
) -> Result<DirPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .remove_session_dir(SessionDirRemoveParams {
            session_id: session_id.clone(),
            path,
        })
        .map(|result| {
            pane_spec(SessionDirListResult {
                revision: result.revision,
                dirs: result.dirs,
            })
        })
}

fn pane_spec(result: SessionDirListResult) -> DirPaneSpec {
    let mut actions = BTreeMap::new();
    let items = result
        .dirs
        .into_iter()
        .enumerate()
        .map(|(index, dir)| {
            let item_id = ListSelectionItemId::new(format!("dir-{index}"));
            actions.insert(
                item_id.clone(),
                DirSelectionAction::Remove {
                    path: dir.path.clone(),
                },
            );
            ListSelectionItem::new(dir.path.display().to_string())
                .with_id(item_id)
                .with_description(if dir.permissions.is_empty() {
                    "no permissions".to_owned()
                } else {
                    dir.permissions
                        .iter()
                        .map(permission_label)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
        })
        .collect();
    DirPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Directories",
                vec![ListSelectionGroup::new("Directories", items)],
            )
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search directories"))
            .with_empty_message("No directories"),
            "↑/↓ search/select  ·  Enter remove  ·  Esc back",
        ),
        actions,
    }
}

fn permission_label(permission: &PermissionDto) -> &'static str {
    match permission {
        PermissionDto::ReadFiles => "read files",
        PermissionDto::WriteFiles => "write files",
        PermissionDto::ExecuteCommands => "execute commands",
        PermissionDto::WatchFiles => "watch changes",
        PermissionDto::BrowseFiles => "browse files",
        PermissionDto::SearchFiles => "search files",
        PermissionDto::LoadInstructions => "instructions and agents",
        PermissionDto::LoadConfig => "config",
        PermissionDto::DiscoverSkills => "skills",
        PermissionDto::DiscoverMcp => "mcp",
        PermissionDto::UseLanguageServices => "lsp",
        PermissionDto::DiscoverHooks => "hooks",
        PermissionDto::DiscoverPlugins => "plugins",
        PermissionDto::InspectRepository => "inspect repository",
        PermissionDto::MutateRepository => "mutate repository",
    }
}

#[cfg(test)]
#[path = "dirs_tests.rs"]
mod tests;
