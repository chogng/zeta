use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use std::path::PathBuf;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryAddParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryMutationResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryRemoveParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AdditionalDirectorySelectionAction {
    Remove { root: PathBuf },
}

pub(crate) struct AdditionalDirectorySelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, AdditionalDirectorySelectionAction>,
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
) -> Result<AdditionalDirectorySelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_workspace_additional_directories(WorkspaceAdditionalDirectoryListParams {
            session_id: session_id.clone(),
        })
        .map(selection_view)
}

pub(crate) fn add<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    root: PathBuf,
) -> Result<WorkspaceAdditionalDirectoryMutationResult, ClientError>
where
    T: JsonRpcTransport,
{
    client.add_workspace_additional_directory(WorkspaceAdditionalDirectoryAddParams {
        session_id: session_id.clone(),
        root,
    })
}

pub(crate) fn remove<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    root: PathBuf,
) -> Result<AdditionalDirectorySelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .remove_workspace_additional_directory(WorkspaceAdditionalDirectoryRemoveParams {
            session_id: session_id.clone(),
            root,
        })
        .map(|result| {
            selection_view(WorkspaceAdditionalDirectoryListResult {
                directories: result.directories,
            })
        })
}

fn selection_view(
    result: WorkspaceAdditionalDirectoryListResult,
) -> AdditionalDirectorySelectionView {
    let mut actions = BTreeMap::new();
    let items = result
        .directories
        .into_iter()
        .enumerate()
        .map(|(index, directory)| {
            let item_id = SelectionItemId::new(format!("additional-directory-{index}"));
            actions.insert(
                item_id.clone(),
                AdditionalDirectorySelectionAction::Remove {
                    root: directory.root.clone(),
                },
            );
            SelectionItem::new(directory.root.display().to_string())
                .with_id(item_id)
                .with_description(match directory.trust {
                    WorkspaceTrustStateDto::Restricted => "restricted file access",
                    WorkspaceTrustStateDto::Trusted => "trusted workspace access",
                })
        })
        .collect();
    AdditionalDirectorySelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Additional directories",
                vec![SelectionTab::new("Directories", items)],
            )
            .with_activation_mode(SelectionActivationMode::Enter)
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search directories"))
            .with_empty_message("No additional directories"),
            "Space search  ·  ↑/↓ select  ·  Enter remove  ·  Esc back",
        ),
        actions,
    }
}

#[cfg(test)]
#[path = "additional_directories_tests.rs"]
mod tests;
