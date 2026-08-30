use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::project_projection;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::projects::ProjectChanged;
use zeta_app_server_protocol::protocol::projects::ProjectCommandDispositionDto;
use zeta_app_server_protocol::protocol::projects::ProjectCreateParams;
use zeta_app_server_protocol::protocol::projects::ProjectDetailsUpdateParams;
use zeta_app_server_protocol::protocol::projects::ProjectLifecycleParams;
use zeta_app_server_protocol::protocol::projects::ProjectListParams;
use zeta_app_server_protocol::protocol::projects::ProjectListResult;
use zeta_app_server_protocol::protocol::projects::ProjectMutationResult;
use zeta_app_server_protocol::protocol::projects::ProjectReadParams;
use zeta_app_server_protocol::protocol::projects::ProjectReadResult;
use zeta_app_server_protocol::protocol::projects::ProjectRootAddParams;
use zeta_app_server_protocol::protocol::projects::ProjectRootRemoveParams;
use zeta_app_server_protocol::protocol::projects::ProjectRootUpdateParams;
use zeta_app_server_protocol::protocol::projects::ProjectSessionMutationParams;
use zeta_app_server_protocol::protocol::projects::ProjectWorkRunMutationParams;
use zeta_projects::ProjectCommand;
use zeta_projects::ProjectCommandDisposition;
use zeta_projects::ProjectCommandRequest;
use zeta_projects::ProjectCommandResult;
use zeta_projects::ProjectCoordinator;
use zeta_projects::ProjectError;
use zeta_projects::ProjectRoot;

impl AppServer {
    pub(super) fn project_list(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let _: ProjectListParams = decode(params)?;
        let projects = self
            .project_coordinator(connection)?
            .list()
            .map_err(project_error)?
            .iter()
            .map(project_projection::summary)
            .collect();
        result(&ProjectListResult { projects })
    }

    pub(super) fn project_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectReadParams = decode(params)?;
        let project = self
            .project_coordinator(connection)?
            .read(&params.project_id)
            .map_err(project_error)?;
        result(&ProjectReadResult {
            project: project_projection::project(&project),
        })
    }

    pub(super) fn project_create(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectCreateParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: 0,
                command: ProjectCommand::Create {
                    name: params.name,
                    description: params.description,
                },
            },
        )
    }

    pub(super) fn project_details_update(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectDetailsUpdateParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::UpdateDetails {
                    name: params.name,
                    description: params.description,
                },
            },
        )
    }

    pub(super) fn project_root_add(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectRootAddParams = decode(params)?;
        let dir = self
            .env_runtime
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .dir_grants
            .list(&params.session_id)
            .into_iter()
            .find(|entry| entry.dir().id() == params.dir_id)
            .map(|entry| entry.dir().clone())
            .ok_or_else(invalid_project_reference)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::AddRoot {
                    root: ProjectRoot {
                        environment_id: dir.env().clone(),
                        dir_id: dir.id(),
                        path: dir.canonical_path().to_path_buf(),
                        name: params.name,
                        purpose: params.purpose,
                    },
                },
            },
        )
    }

    pub(super) fn project_root_update(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectRootUpdateParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::UpdateRootDetails {
                    dir_id: params.dir_id,
                    name: params.name,
                    purpose: params.purpose,
                },
            },
        )
    }

    pub(super) fn project_root_remove(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectRootRemoveParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::RemoveRoot {
                    dir_id: params.dir_id,
                },
            },
        )
    }

    pub(super) fn project_session_link(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectSessionMutationParams = decode(params)?;
        if self
            .threads
            .list_session_threads(&params.session_id)
            .map_err(|_| invalid_project_reference())?
            .is_empty()
        {
            return Err(invalid_project_reference());
        }
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::LinkSession {
                    session_id: params.session_id,
                },
            },
        )
    }

    pub(super) fn project_session_unlink(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectSessionMutationParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::UnlinkSession {
                    session_id: params.session_id,
                },
            },
        )
    }

    pub(super) fn project_work_run_link(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectWorkRunMutationParams = decode(params)?;
        self.work_coordination
            .as_deref()
            .ok_or_else(projects_unavailable)?
            .read(&params.work_run_id)
            .map_err(|_| invalid_project_reference())?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::LinkWorkRun {
                    work_run_id: params.work_run_id,
                },
            },
        )
    }

    pub(super) fn project_work_run_unlink(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ProjectWorkRunMutationParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command: ProjectCommand::UnlinkWorkRun {
                    work_run_id: params.work_run_id,
                },
            },
        )
    }

    pub(super) fn project_archive(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        self.apply_project_lifecycle(connection, params, ProjectCommand::Archive)
    }

    pub(super) fn project_restore(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        self.apply_project_lifecycle(connection, params, ProjectCommand::Restore)
    }

    fn apply_project_lifecycle(
        &self,
        connection: &ConnectionState,
        params: &Value,
        command: ProjectCommand,
    ) -> Result<Value, RpcError> {
        let params: ProjectLifecycleParams = decode(params)?;
        self.apply_project(
            connection,
            ProjectCommandRequest {
                command_id: params.command_id,
                project_id: params.project_id,
                expected_revision: params.expected_revision,
                command,
            },
        )
    }

    fn apply_project(
        &self,
        connection: &ConnectionState,
        request: ProjectCommandRequest,
    ) -> Result<Value, RpcError> {
        let command = self
            .project_coordinator(connection)?
            .apply(request)
            .map_err(project_error)?;
        let response = mutation_result(&command);
        if command.disposition == ProjectCommandDisposition::Committed {
            self.updates.publish_project_changed(ProjectChanged {
                project: response.project.clone(),
            });
        }
        result(&response)
    }

    fn project_coordinator(
        &self,
        connection: &ConnectionState,
    ) -> Result<&ProjectCoordinator, RpcError> {
        if !connection.supports_work_coordination_host() {
            return Err(RpcError::new(
                -32073,
                AppServerErrorName::PermissionRequired,
            ));
        }
        self.projects.as_deref().ok_or_else(projects_unavailable)
    }
}

fn mutation_result(command: &ProjectCommandResult) -> ProjectMutationResult {
    ProjectMutationResult {
        disposition: match command.disposition {
            ProjectCommandDisposition::Committed => ProjectCommandDispositionDto::Committed,
            ProjectCommandDisposition::Replayed => ProjectCommandDispositionDto::Replayed,
        },
        project: project_projection::project(&command.project),
    }
}

fn projects_unavailable() -> RpcError {
    RpcError::new(-32094, AppServerErrorName::ProjectsUnavailable)
}

fn invalid_project_reference() -> RpcError {
    RpcError::new(-32602, AppServerErrorName::InvalidParams)
}

fn project_error(error: ProjectError) -> RpcError {
    match error {
        ProjectError::NotFound(_) => RpcError::new(-32095, AppServerErrorName::ProjectNotFound),
        ProjectError::RevisionConflict { .. } => {
            RpcError::new(-32096, AppServerErrorName::ProjectRevisionConflict)
        }
        ProjectError::CommandConflict => RpcError::new(-32012, AppServerErrorName::CommandConflict),
        ProjectError::AlreadyExists(_)
        | ProjectError::InvalidInput(_)
        | ProjectError::InvalidTransition(_)
        | ProjectError::Storage(_) => {
            RpcError::new(-32097, AppServerErrorName::ProjectOperationFailed)
        }
    }
}
