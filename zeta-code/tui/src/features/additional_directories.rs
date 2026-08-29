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
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryAddParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryMutationResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryRemoveParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AdditionalDirectorySelectionAction {
    Remove { root: PathBuf },
}

pub(crate) struct AdditionalDirectoryPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, AdditionalDirectorySelectionAction>,
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
) -> Result<AdditionalDirectoryPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_workspace_additional_directories(WorkspaceAdditionalDirectoryListParams {
            session_id: session_id.clone(),
        })
        .map(list_selection)
}

pub(crate) fn add<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    root: PathBuf,
    permissions: Vec<WorkspaceAdditionalDirectoryPermissionDto>,
) -> Result<WorkspaceAdditionalDirectoryMutationResult, ClientError>
where
    T: JsonRpcTransport,
{
    client.add_workspace_additional_directory(WorkspaceAdditionalDirectoryAddParams {
        session_id: session_id.clone(),
        root,
        permissions,
    })
}

pub(crate) fn remove<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    root: PathBuf,
) -> Result<AdditionalDirectoryPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .remove_workspace_additional_directory(WorkspaceAdditionalDirectoryRemoveParams {
            session_id: session_id.clone(),
            root,
        })
        .map(|result| {
            list_selection(WorkspaceAdditionalDirectoryListResult {
                revision: result.revision,
                directories: result.directories,
            })
        })
}

fn list_selection(result: WorkspaceAdditionalDirectoryListResult) -> AdditionalDirectoryPaneSpec {
    let mut actions = BTreeMap::new();
    let items = result
        .directories
        .into_iter()
        .enumerate()
        .map(|(index, directory)| {
            let item_id = ListSelectionItemId::new(format!("additional-directory-{index}"));
            actions.insert(
                item_id.clone(),
                AdditionalDirectorySelectionAction::Remove {
                    root: directory.root.clone(),
                },
            );
            ListSelectionItem::new(directory.root.display().to_string())
                .with_id(item_id)
                .with_description(format!(
                    "{} · {}",
                    match directory.trust {
                        WorkspaceTrustStateDto::Restricted => "restricted",
                        WorkspaceTrustStateDto::Trusted => "trusted",
                    },
                    if directory.permissions.is_empty() {
                        "no permissions".to_owned()
                    } else {
                        directory
                            .permissions
                            .iter()
                            .map(permission_label)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ))
        })
        .collect();
    AdditionalDirectoryPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Additional directories",
                vec![ListSelectionGroup::new("Directories", items)],
            )
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search directories"))
            .with_empty_message("No additional directories"),
            "Space search  ·  ↑/↓ select  ·  Enter remove  ·  Esc back",
        ),
        actions,
    }
}

fn permission_label(permission: &WorkspaceAdditionalDirectoryPermissionDto) -> &'static str {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => "read files",
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => "write files",
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => "execute commands",
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => "watch changes",
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles => "workspace files",
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch => "workspace search",
        WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents => {
            "instructions and agents"
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills => "skills",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp => "mcp",
        WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices => "lsp",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks => "hooks",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins => "plugins",
    }
}

#[cfg(test)]
#[path = "additional_directories_tests.rs"]
mod tests;
