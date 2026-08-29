use super::RequestCompletion;
use super::spawn_request;
use crate::app::App;
use crate::app::AppEvent;
use crate::client;
use crate::features::config;
use crate::features::config::AdditionalDirectoryPermissionEdit;
use crate::features::config::ConfigEdit;
use crate::features::config::ConfigResource;
use crate::features::config::ProviderApiKeyEdit;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ProviderApiKeySetRequest;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_protocol::SessionId;

pub(super) fn open_config(
    resource: &mut Option<ConfigResource>,
    session_id: &SessionId,
    client: &AppServerRequestHandle,
    pending_request: &mut Option<client::RequestTask<RequestCompletion>>,
    app: &mut App,
) {
    let terminal_snapshot = match resource.as_mut() {
        Some(resource) => match resource.refresh() {
            Ok(settings) => {
                app.update(AppEvent::ConfigSettingsReceived(settings));
                Some((settings, resource.revision()))
            }
            Err(error) => {
                app.update(AppEvent::FailureReported(error));
                None
            }
        },
        None => Some((config::TerminalSettings::default(), 0)),
    };
    if let Some((settings, revision)) = terminal_snapshot
        && pending_request.is_none()
    {
        let mut request_client = client.clone();
        let session_id = session_id.clone();
        *pending_request = spawn_request(
            "zeta-tui-read-config",
            move || {
                RequestCompletion::Presentation(
                    request_client
                        .read_config()
                        .and_then(|server_config| {
                            request_client.list_providers().and_then(|providers| {
                                request_client
                                    .list_workspace_additional_directories(
                                        WorkspaceAdditionalDirectoryListParams {
                                            session_id: session_id.clone(),
                                        },
                                    )
                                    .map(|additional_directories| {
                                        AppEvent::ConfigPaneOpened(config::config_pane_spec(
                                            &server_config,
                                            &providers,
                                            settings,
                                            revision,
                                            &session_id,
                                            &additional_directories,
                                        ))
                                    })
                            })
                        })
                        .map_err(|error| error.to_string()),
                )
            },
            app,
        );
    }
}

pub(super) fn edit_config(resource: &mut Option<ConfigResource>, edit: &ConfigEdit, app: &mut App) {
    match resource.as_mut() {
        Some(resource) => match resource.apply_edit(&edit.terminal) {
            Ok((settings, revision)) => {
                let view = config::config_pane_spec(
                    &edit.server_config,
                    &edit.providers,
                    settings,
                    revision,
                    &edit.session_id,
                    &edit.additional_directories,
                );
                app.update(AppEvent::ConfigSettingsReceived(settings));
                app.update(AppEvent::ConfigPaneReplaced(view));
            }
            Err(error) => app.update(AppEvent::FailureReported(error)),
        },
        None => app.update(AppEvent::FailureReported(
            "terminal settings are unavailable because no active profile root was configured"
                .to_owned(),
        )),
    }
}

pub(super) fn set_provider_api_key(
    edit: ProviderApiKeyEdit,
    terminal_snapshot: (config::TerminalSettings, u64),
    session_id: &SessionId,
    client: &AppServerRequestHandle,
    pending_request: &mut Option<client::RequestTask<RequestCompletion>>,
    app: &mut App,
) {
    if pending_request.is_some() {
        return;
    }
    let (provider, api_key) = edit.into_parts();
    let notice_provider = provider.clone();
    let (settings, revision) = terminal_snapshot;
    let mut request_client = client.clone();
    let session_id = session_id.clone();
    *pending_request = spawn_request(
        "zeta-tui-set-provider-api-key",
        move || {
            RequestCompletion::Presentation(
                request_client
                    .set_provider_api_key(ProviderApiKeySetRequest::new(provider, api_key))
                    .and_then(|_| request_client.read_config())
                    .and_then(|server_config| {
                        request_client.list_providers().and_then(|providers| {
                            request_client
                                .list_workspace_additional_directories(
                                    WorkspaceAdditionalDirectoryListParams {
                                        session_id: session_id.clone(),
                                    },
                                )
                                .map(|additional_directories| AppEvent::ConfigApiKeySaved {
                                    provider: notice_provider,
                                    pane_spec: config::config_pane_spec(
                                        &server_config,
                                        &providers,
                                        settings,
                                        revision,
                                        &session_id,
                                        &additional_directories,
                                    ),
                                })
                        })
                    })
                    .map_err(|error| error.to_string()),
            )
        },
        app,
    );
}

pub(super) fn set_additional_directory_permissions(
    edit: AdditionalDirectoryPermissionEdit,
    client: &AppServerRequestHandle,
    pending_request: &mut Option<client::RequestTask<RequestCompletion>>,
    app: &mut App,
) {
    if pending_request.is_some() {
        return;
    }
    let mut request_client = client.clone();
    *pending_request = spawn_request(
        "zeta-tui-set-additional-directory-permissions",
        move || {
            RequestCompletion::Presentation(
                request_client
                    .set_workspace_additional_directory_permissions(edit.params.clone())
                    .map(|result| {
                        AppEvent::ConfigPaneReplaced(config::config_pane_spec(
                            &edit.server_config,
                            &edit.providers,
                            edit.terminal,
                            edit.terminal_revision,
                            &edit.params.session_id,
                            &WorkspaceAdditionalDirectoryListResult {
                                revision: result.revision,
                                directories: result.directories,
                            },
                        ))
                    })
                    .map_err(|error| error.to_string()),
            )
        },
        app,
    );
}
