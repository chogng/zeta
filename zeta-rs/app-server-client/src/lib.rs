//! Reusable typed app-server client boundary and contract-test entry point.

mod in_process;
mod notification;
mod profile;
mod session;

use serde_json::Value;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::config::{
    ConfigCommandResult, ConfigReadResult, ConfigUpdateParams, LanguageServerConfigureParams,
    LanguageServerRemoveParams, McpServerRemoveParams, McpServerSetEnablementParams,
    McpServerUpsertParams, ProviderConfigureParams, ProviderRemoveParams, SkillSourceAddParams,
    SkillSourceRemoveParams, SkillSourceSetEnablementParams,
};
use zeta_app_server_protocol::protocol::document::{TypstCompileParams, TypstCompileResult};
use zeta_app_server_protocol::protocol::fs::{
    FsGetMetadataParams, FsGetMetadataResult, FsReadDirectoryParams, FsReadDirectoryResult,
    FsReadFileParams, FsReadFileResult, FsWriteFileParams, FsWriteFileResult,
};
use zeta_app_server_protocol::protocol::git::{
    GitBranchListResult, GitBranchSwitchParams, GitOperationResult, GitTextDiffResult,
};
use zeta_app_server_protocol::protocol::initialize::{InitializeParams, InitializeResult};
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_app_server_protocol::protocol::registry::ClientMethod;
use zeta_app_server_protocol::protocol::resources::{
    ResourceMetadataParams, ResourceMetadataResult, ResourceReadParams, ResourceReadResult,
    ResourceReleaseParams,
};
use zeta_app_server_protocol::protocol::search::{
    WorkspaceSearchCancelParams, WorkspaceSearchReadParams, WorkspaceSearchReadResult,
    WorkspaceSearchStartParams, WorkspaceSearchStartResult,
};
use zeta_app_server_protocol::protocol::session::{
    SessionCommandParams, SessionCreateParams, SessionListResult, SessionModelSetParams,
    SessionReadParams, SessionResult, SessionSubscribeParams, SessionSubscribeResult,
    SessionThreadArchiveParams, SessionThreadCreateParams, SessionThreadForkParams,
    SessionThreadResult, SessionThreadRewindParams, SessionUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::skills::{
    SkillListParams, SkillListResult, SkillSetEnablementParams,
};
use zeta_app_server_protocol::protocol::thread::{
    ThreadReadParams, ThreadReadResult, ThreadSubscribeParams, ThreadSubscribeResult,
    ThreadUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::turn::{
    ShellTurnStartParams, TurnInteractionResolveParams, TurnInteractionResolveResult,
    TurnInterruptParams, TurnInterruptResult, TurnStartParams, TurnStartResult,
};
use zeta_app_server_protocol::protocol::workspace::{WorkspaceSwitchParams, WorkspaceSwitchResult};
use zeta_app_server_protocol::rpc::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

pub use in_process::InProcessAppServer;
pub use in_process::InProcessClientOptions;
pub use in_process::InProcessTransport;
pub use in_process::open_in_process_app_server;
pub use in_process::start_in_process_client;
pub use notification::ServerNotification;
pub use profile::local_profile_root;
pub use session::{
    AppServerEvent, AppServerEvents, AppServerRequestHandle, AppServerSession,
    ConnectionCloseReason, ShutdownError, TakeEventsError,
};

/// Exchanges one complete JSON-RPC request with a connected app-server transport.
///
/// Implementations must preserve request/response pairing for a single connection and must not
/// return notifications as if they were responses to this client call. Implementations buffer
/// causally emitted notifications until `drain_notifications` is called.
pub trait JsonRpcTransport {
    fn round_trip(&mut self, request: &str) -> Result<String, ClientError>;

    fn drain_notifications(&mut self) -> Result<Vec<String>, ClientError> {
        Ok(Vec::new())
    }
}

pub struct AppServerClient<T> {
    transport: T,
    next_request_id: Arc<AtomicU64>,
    initialization: Arc<OnceLock<InitializeResult>>,
}

impl<T: Clone> Clone for AppServerClient<T> {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            next_request_id: Arc::clone(&self.next_request_id),
            initialization: Arc::clone(&self.initialization),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    Transport(String),
    Protocol(String),
    Server { code: i64, message: String },
}

impl<T: JsonRpcTransport> AppServerClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_request_id: Arc::new(AtomicU64::new(1)),
            initialization: Arc::new(OnceLock::new()),
        }
    }

    pub fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<InitializeResult, ClientError> {
        let initialization: InitializeResult = self.call(ClientMethod::Initialize, params)?;
        self.initialization
            .set(initialization.clone())
            .map_err(|_| {
                ClientError::Protocol("App Server client is already initialized".into())
            })?;
        Ok(initialization)
    }

    /// Returns the immutable server snapshot captured by the successful initialize handshake.
    pub fn initialization(&self) -> Result<&InitializeResult, ClientError> {
        self.initialization.get().ok_or_else(|| {
            ClientError::Protocol(
                "App Server client has not completed the initialize handshake".into(),
            )
        })
    }

    pub fn switch_workspace(
        &mut self,
        params: WorkspaceSwitchParams,
    ) -> Result<WorkspaceSwitchResult, ClientError> {
        self.call(ClientMethod::WorkspaceSwitch, params)
    }

    pub fn read_directory(
        &mut self,
        params: FsReadDirectoryParams,
    ) -> Result<FsReadDirectoryResult, ClientError> {
        self.call(ClientMethod::FsReadDirectory, params)
    }

    pub fn get_file_metadata(
        &mut self,
        params: FsGetMetadataParams,
    ) -> Result<FsGetMetadataResult, ClientError> {
        self.call(ClientMethod::FsGetMetadata, params)
    }

    pub fn read_file(&mut self, params: FsReadFileParams) -> Result<FsReadFileResult, ClientError> {
        self.call(ClientMethod::FsReadFile, params)
    }

    pub fn write_file(
        &mut self,
        params: FsWriteFileParams,
    ) -> Result<FsWriteFileResult, ClientError> {
        self.call(ClientMethod::FsWriteFile, params)
    }

    pub fn git_text_diff(&mut self) -> Result<GitTextDiffResult, ClientError> {
        self.call(ClientMethod::GitTextDiff, EmptyParams {})
    }

    pub fn list_git_branches(&mut self) -> Result<GitBranchListResult, ClientError> {
        self.call(ClientMethod::GitBranchList, EmptyParams {})
    }

    pub fn switch_git_branch(
        &mut self,
        params: GitBranchSwitchParams,
    ) -> Result<GitOperationResult, ClientError> {
        self.call(ClientMethod::GitBranchSwitch, params)
    }

    pub fn create_session(
        &mut self,
        params: SessionCreateParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionCreate, params)
    }

    pub fn read_session(
        &mut self,
        params: SessionReadParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionRead, params)
    }

    pub fn list_sessions(&mut self) -> Result<SessionListResult, ClientError> {
        self.call(ClientMethod::SessionList, EmptyParams {})
    }

    pub fn set_session_model(
        &mut self,
        params: SessionModelSetParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionModelSet, params)
    }

    pub fn subscribe_session(
        &mut self,
        params: SessionSubscribeParams,
    ) -> Result<SessionSubscribeResult, ClientError> {
        self.call(ClientMethod::SessionSubscribe, params)
    }

    pub fn unsubscribe_session(
        &mut self,
        params: SessionUnsubscribeParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::SessionUnsubscribe, params)
    }

    pub fn create_session_thread(
        &mut self,
        params: SessionThreadCreateParams,
    ) -> Result<SessionThreadResult, ClientError> {
        self.call(ClientMethod::SessionThreadCreate, params)
    }

    pub fn fork_session_thread(
        &mut self,
        params: SessionThreadForkParams,
    ) -> Result<SessionThreadResult, ClientError> {
        self.call(ClientMethod::SessionThreadFork, params)
    }

    pub fn rewind_session_thread(
        &mut self,
        params: SessionThreadRewindParams,
    ) -> Result<SessionThreadResult, ClientError> {
        self.call(ClientMethod::SessionThreadRewind, params)
    }

    pub fn archive_session_thread(
        &mut self,
        params: SessionThreadArchiveParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionThreadArchive, params)
    }

    pub fn complete_session(
        &mut self,
        params: SessionCommandParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionComplete, params)
    }

    pub fn archive_session(
        &mut self,
        params: SessionCommandParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionArchive, params)
    }

    pub fn stop_session(
        &mut self,
        params: SessionCommandParams,
    ) -> Result<SessionResult, ClientError> {
        self.call(ClientMethod::SessionStop, params)
    }

    pub fn read_thread(
        &mut self,
        params: ThreadReadParams,
    ) -> Result<ThreadReadResult, ClientError> {
        self.call(ClientMethod::ThreadRead, params)
    }

    pub fn subscribe_thread(
        &mut self,
        params: ThreadSubscribeParams,
    ) -> Result<ThreadSubscribeResult, ClientError> {
        self.call(ClientMethod::ThreadSubscribe, params)
    }

    pub fn unsubscribe_thread(
        &mut self,
        params: ThreadUnsubscribeParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::ThreadUnsubscribe, params)
    }

    pub fn read_config(&mut self) -> Result<ConfigReadResult, ClientError> {
        self.call(ClientMethod::ConfigRead, EmptyParams {})
    }

    pub fn list_models(&mut self) -> Result<ModelListResult, ClientError> {
        self.call(ClientMethod::ModelList, EmptyParams {})
    }

    pub fn update_config(
        &mut self,
        params: ConfigUpdateParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::ConfigUpdate, params)
    }

    pub fn configure_language_server(
        &mut self,
        params: LanguageServerConfigureParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::LanguageServerConfigure, params)
    }

    pub fn remove_language_server_configuration(
        &mut self,
        params: LanguageServerRemoveParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::LanguageServerRemove, params)
    }

    pub fn configure_provider(
        &mut self,
        params: ProviderConfigureParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::ProviderConfigure, params)
    }

    pub fn remove_provider(
        &mut self,
        params: ProviderRemoveParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::ProviderRemove, params)
    }

    pub fn upsert_mcp_server(
        &mut self,
        params: McpServerUpsertParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::McpServerUpsert, params)
    }

    pub fn remove_mcp_server(
        &mut self,
        params: McpServerRemoveParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::McpServerRemove, params)
    }

    pub fn set_mcp_server_enablement(
        &mut self,
        params: McpServerSetEnablementParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::McpServerSetEnablement, params)
    }

    pub fn add_skill_source(
        &mut self,
        params: SkillSourceAddParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::SkillSourceAdd, params)
    }

    pub fn remove_skill_source(
        &mut self,
        params: SkillSourceRemoveParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::SkillSourceRemove, params)
    }

    pub fn set_skill_source_enablement(
        &mut self,
        params: SkillSourceSetEnablementParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::SkillSourceSetEnablement, params)
    }

    pub fn list_skills(&mut self, params: SkillListParams) -> Result<SkillListResult, ClientError> {
        self.call(ClientMethod::SkillList, params)
    }

    pub fn set_skill_enablement(
        &mut self,
        params: SkillSetEnablementParams,
    ) -> Result<ConfigCommandResult, ClientError> {
        self.call(ClientMethod::SkillSetEnablement, params)
    }

    pub fn start_turn(&mut self, params: TurnStartParams) -> Result<TurnStartResult, ClientError> {
        self.call(ClientMethod::TurnStart, params)
    }

    pub fn start_shell_turn(
        &mut self,
        params: ShellTurnStartParams,
    ) -> Result<TurnStartResult, ClientError> {
        self.call(ClientMethod::ShellTurnStart, params)
    }

    pub fn interrupt_turn(
        &mut self,
        params: TurnInterruptParams,
    ) -> Result<TurnInterruptResult, ClientError> {
        self.call(ClientMethod::TurnInterrupt, params)
    }

    pub fn resolve_turn_interaction(
        &mut self,
        params: TurnInteractionResolveParams,
    ) -> Result<TurnInteractionResolveResult, ClientError> {
        self.call(ClientMethod::TurnInteractionResolve, params)
    }

    pub fn compile_typst(
        &mut self,
        params: TypstCompileParams,
    ) -> Result<TypstCompileResult, ClientError> {
        self.call(ClientMethod::TypstCompile, params)
    }

    pub fn resource_metadata(
        &mut self,
        params: ResourceMetadataParams,
    ) -> Result<ResourceMetadataResult, ClientError> {
        self.call(ClientMethod::ResourceMetadata, params)
    }

    pub fn read_resource(
        &mut self,
        params: ResourceReadParams,
    ) -> Result<ResourceReadResult, ClientError> {
        self.call(ClientMethod::ResourceRead, params)
    }

    pub fn release_resource(&mut self, params: ResourceReleaseParams) -> Result<(), ClientError> {
        self.call(ClientMethod::ResourceRelease, params)
    }

    pub fn start_workspace_search(
        &mut self,
        params: WorkspaceSearchStartParams,
    ) -> Result<WorkspaceSearchStartResult, ClientError> {
        self.call(ClientMethod::WorkspaceSearchStart, params)
    }

    pub fn read_workspace_search(
        &mut self,
        params: WorkspaceSearchReadParams,
    ) -> Result<WorkspaceSearchReadResult, ClientError> {
        self.call(ClientMethod::WorkspaceSearchRead, params)
    }

    pub fn cancel_workspace_search(
        &mut self,
        params: WorkspaceSearchCancelParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::WorkspaceSearchCancel, params)
    }

    pub fn drain_notifications(&mut self) -> Result<Vec<ServerNotification>, ClientError> {
        self.transport
            .drain_notifications()?
            .into_iter()
            .map(|raw| notification::decode(&raw))
            .collect()
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    fn call<P: serde::Serialize, R: for<'a> serde::Deserialize<'a>>(
        &mut self,
        method: ClientMethod,
        params: P,
    ) -> Result<R, ClientError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let params = serde_json::to_value(params)
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(request_id),
            method.as_str().into(),
            params,
        );
        let encoded_request = serde_json::to_string(&request)
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        let raw_response = self.transport.round_trip(&encoded_request)?;
        let response: JsonRpcResponse<Value, Value> = serde_json::from_str(&raw_response)
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        let response_id = match &response {
            JsonRpcResponse::Success(response) => &response.id,
            JsonRpcResponse::Failure(response) => &response.id,
        };
        if response_id != &JsonRpcId::Number(request_id) {
            return Err(ClientError::Protocol(
                "response id did not match request".into(),
            ));
        }
        match response {
            JsonRpcResponse::Success(response) => serde_json::from_value(response.result)
                .map_err(|error| ClientError::Protocol(error.to_string())),
            JsonRpcResponse::Failure(response) => Err(ClientError::Server {
                code: response
                    .error
                    .get("code")
                    .and_then(Value::as_i64)
                    .unwrap_or(-32000),
                message: response
                    .error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .into(),
            }),
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => write!(formatter, "transport error: {message}"),
            Self::Protocol(message) => write!(formatter, "protocol error: {message}"),
            Self::Server { code, message } => {
                write!(formatter, "server error {code}: {message}")
            }
        }
    }
}

impl std::error::Error for ClientError {}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
