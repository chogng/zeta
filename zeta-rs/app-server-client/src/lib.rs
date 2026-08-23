//! Reusable typed app-server client boundary and contract-test entry point.

mod in_process;
mod notification;
mod product_services;
mod profile;
mod session;
mod session_workspace;

use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeStruct;
use serde_json::Value;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use zeroize::Zeroize;
use zeroize::Zeroizing;
use zeta_app_server_protocol::protocol::attachments::AttachmentImportRemoteParams;
use zeta_app_server_protocol::protocol::attachments::AttachmentMaterializeResult;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadCancelParams;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadFinishParams;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadStartParams;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadStartResult;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadWriteParams;
use zeta_app_server_protocol::protocol::attachments::AttachmentUploadWriteResult;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::config::{
    ConfigCommandResult, ConfigReadResult, ConfigUpdateParams, LanguageServerConfigureParams,
    LanguageServerRemoveParams, McpServerRemoveParams, McpServerSetEnablementParams,
    McpServerUpsertParams, ProviderConfigureParams, ProviderRemoveParams, SkillSourceAddParams,
    SkillSourceRemoveParams, SkillSourceSetEnablementParams,
};
use zeta_app_server_protocol::protocol::connectors::ConnectorCommandResultDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorCredentialCleanupDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorCredentialCleanupParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorDeviceOAuthPollParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorDeviceOAuthPollResult;
use zeta_app_server_protocol::protocol::connectors::ConnectorDeviceOAuthStartParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorDeviceOAuthStartResult;
use zeta_app_server_protocol::protocol::connectors::ConnectorDisconnectParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorDisconnectResultDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorListResult;
use zeta_app_server_protocol::protocol::connectors::ConnectorOAuthCancelParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorOAuthRefreshParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorOAuthStartParams;
use zeta_app_server_protocol::protocol::connectors::ConnectorOAuthStartResult;
use zeta_app_server_protocol::protocol::diff::DiffComputeParams;
use zeta_app_server_protocol::protocol::diff::DiffComputeResult;
use zeta_app_server_protocol::protocol::document::{TypstCompileParams, TypstCompileResult};
use zeta_app_server_protocol::protocol::fs::{
    FsGetMetadataParams, FsGetMetadataResult, FsReadBinaryFileParams, FsReadBinaryFileResult,
    FsReadDirectoryParams, FsReadDirectoryResult, FsReadFileParams, FsReadFileResult,
    FsWriteFileParams, FsWriteFileResult,
};
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_app_server_protocol::protocol::git::{
    GitBranchListResult, GitBranchSwitchParams, GitOperationResult, GitTextDiffResult,
};
use zeta_app_server_protocol::protocol::initialize::{InitializeParams, InitializeResult};
use zeta_app_server_protocol::protocol::language::LanguageCloseParams;
use zeta_app_server_protocol::protocol::language::LanguageCompletionsParams;
use zeta_app_server_protocol::protocol::language::LanguageCompletionsResult;
use zeta_app_server_protocol::protocol::language::LanguageHoverParams;
use zeta_app_server_protocol::protocol::language::LanguageHoverResult;
use zeta_app_server_protocol::protocol::language::LanguageLocationsParams;
use zeta_app_server_protocol::protocol::language::LanguageLocationsResult;
use zeta_app_server_protocol::protocol::language::LanguageSynchronizeParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceAcquireCapabilityParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceAcquiredCapabilityDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceArtifactHandleDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceDownloadParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceGetParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceInstallParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceInstalledPackageDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceListInstalledResult;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceOpenResourceParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplacePackageDetailsDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceReleaseCapabilityParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceResourceContentDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceSearchParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceSearchResult;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUninstallParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUpdateParams;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_app_server_protocol::protocol::plugins::PluginCommandResultDto;
use zeta_app_server_protocol::protocol::plugins::PluginListResult;
use zeta_app_server_protocol::protocol::plugins::PluginPackageCommandParams;
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
    SessionCreateParams, SessionListResult, SessionReadParams, SessionRequestParams,
    SessionRequestResult, SessionResult, SessionSubscribeParams, SessionSubscribeResult,
    SessionThreadReadParams, SessionThreadReadResult, SessionThreadSubscribeParams,
    SessionThreadSubscribeResult, SessionThreadUnsubscribeParams, SessionUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::skills::{
    SkillListParams, SkillListResult, SkillResourceOpenParams, SkillResourceOpenResult,
    SkillSetEnablementParams,
};
use zeta_app_server_protocol::protocol::syntax::SyntaxAnalyzeParams;
use zeta_app_server_protocol::protocol::syntax::SyntaxAnalyzeResult;
use zeta_app_server_protocol::protocol::terminal::TerminalAttachParams;
use zeta_app_server_protocol::protocol::terminal::TerminalAttachResult;
use zeta_app_server_protocol::protocol::terminal::TerminalCloseParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCreateParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCreateResult;
use zeta_app_server_protocol::protocol::terminal::TerminalProfileListResult;
use zeta_app_server_protocol::protocol::terminal::TerminalReadParams;
use zeta_app_server_protocol::protocol::terminal::TerminalReadResult;
use zeta_app_server_protocol::protocol::terminal::TerminalResizeParams;
use zeta_app_server_protocol::protocol::terminal::TerminalWriteParams;
use zeta_app_server_protocol::protocol::workspace::{WorkspaceSwitchParams, WorkspaceSwitchResult};
use zeta_app_server_protocol::rpc::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

pub use in_process::InProcessAppServer;
pub use in_process::InProcessClientOptions;
pub use in_process::InProcessTransport;
pub use in_process::open_in_process_app_server;
pub use in_process::start_in_process_client;
pub use notification::ServerNotification;
pub use product_services::discovered_product_services_path;
pub use product_services::load_discovered_product_services;
pub use profile::local_profile_root;
pub use session::{
    AppServerEvent, AppServerEvents, AppServerRequestHandle, AppServerSession,
    ConnectionCloseReason, ShutdownError, StdioAppServerCommand, TakeEventsError,
};
pub use session_workspace::SessionWorkspaceRoute;
pub use session_workspace::route_session_workspace;
pub use zeta_app_server::SessionStateMode;

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

/// Client-owned API-token request whose secret and encoded wire buffer are cleared after one call.
pub struct ConnectorApiTokenConnectRequest {
    pub command_id: String,
    pub expected_generation: u64,
    pub connector_id: String,
    pub connection_generation: u64,
    pub account_id: String,
    pub account_display_name: String,
    api_token: Zeroizing<String>,
}

impl ConnectorApiTokenConnectRequest {
    pub fn new(
        command_id: String,
        expected_generation: u64,
        connector_id: String,
        connection_generation: u64,
        account_id: String,
        account_display_name: String,
        api_token: String,
    ) -> Self {
        Self {
            command_id,
            expected_generation,
            connector_id,
            connection_generation,
            account_id,
            account_display_name,
            api_token: Zeroizing::new(api_token),
        }
    }
}

impl fmt::Debug for ConnectorApiTokenConnectRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectorApiTokenConnectRequest")
            .field("command_id", &self.command_id)
            .field("expected_generation", &self.expected_generation)
            .field("connector_id", &self.connector_id)
            .field("connection_generation", &self.connection_generation)
            .field("account_id", &self.account_id)
            .field("account_display_name", &self.account_display_name)
            .field("api_token", &"[REDACTED]")
            .finish()
    }
}

impl Serialize for ConnectorApiTokenConnectRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut request = serializer.serialize_struct("ConnectorApiTokenConnectRequest", 7)?;
        request.serialize_field("commandId", &self.command_id)?;
        request.serialize_field("expectedGeneration", &self.expected_generation)?;
        request.serialize_field("connectorId", &self.connector_id)?;
        request.serialize_field("connectionGeneration", &self.connection_generation)?;
        request.serialize_field("accountId", &self.account_id)?;
        request.serialize_field("accountDisplayName", &self.account_display_name)?;
        request.serialize_field("apiToken", self.api_token.as_str())?;
        request.end()
    }
}

/// Client-owned OAuth callback whose state, code, and encoded wire buffer are one-shot.
pub struct ConnectorOAuthCompleteRequest {
    pub flow_id: String,
    state: Zeroizing<String>,
    authorization_code: Zeroizing<String>,
}

impl ConnectorOAuthCompleteRequest {
    pub fn new(flow_id: String, state: String, authorization_code: String) -> Self {
        Self {
            flow_id,
            state: Zeroizing::new(state),
            authorization_code: Zeroizing::new(authorization_code),
        }
    }
}

impl fmt::Debug for ConnectorOAuthCompleteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectorOAuthCompleteRequest")
            .field("flow_id", &self.flow_id)
            .field("state", &"[REDACTED]")
            .field("authorization_code", &"[REDACTED]")
            .finish()
    }
}

impl Serialize for ConnectorOAuthCompleteRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut request = serializer.serialize_struct("ConnectorOAuthCompleteRequest", 3)?;
        request.serialize_field("flowId", &self.flow_id)?;
        request.serialize_field("state", self.state.as_str())?;
        request.serialize_field("authorizationCode", self.authorization_code.as_str())?;
        request.end()
    }
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

    /// Synchronizes one authoritative editor snapshot with the App Server language runtime.
    pub fn synchronize_language_document(
        &mut self,
        params: LanguageSynchronizeParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::LanguageSynchronize, params)
    }

    /// Releases one document from the App Server language runtime.
    pub fn close_language_document(
        &mut self,
        params: LanguageCloseParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::LanguageClose, params)
    }

    /// Requests hover content for one exact synchronized document revision.
    pub fn language_hover(
        &mut self,
        params: LanguageHoverParams,
    ) -> Result<LanguageHoverResult, ClientError> {
        self.call(ClientMethod::LanguageHover, params)
    }

    /// Requests completion candidates for one exact synchronized document revision.
    pub fn language_completions(
        &mut self,
        params: LanguageCompletionsParams,
    ) -> Result<LanguageCompletionsResult, ClientError> {
        self.call(ClientMethod::LanguageCompletions, params)
    }

    /// Requests cross-file locations from the App Server language runtime.
    pub fn language_locations(
        &mut self,
        params: LanguageLocationsParams,
    ) -> Result<LanguageLocationsResult, ClientError> {
        self.call(ClientMethod::LanguageLocations, params)
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

    pub fn read_binary_file(
        &mut self,
        params: FsReadBinaryFileParams,
    ) -> Result<FsReadBinaryFileResult, ClientError> {
        self.call(ClientMethod::FsReadBinaryFile, params)
    }

    pub fn write_file(
        &mut self,
        params: FsWriteFileParams,
    ) -> Result<FsWriteFileResult, ClientError> {
        self.call(ClientMethod::FsWriteFile, params)
    }

    /// Lists the trusted shell profiles exposed by this App Server connection.
    pub fn terminal_profile_list(&mut self) -> Result<TerminalProfileListResult, ClientError> {
        self.call(ClientMethod::TerminalProfileList, EmptyParams {})
    }

    /// Creates one App Server-owned interactive terminal at the current Workspace root.
    pub fn terminal_create(
        &mut self,
        params: TerminalCreateParams,
    ) -> Result<TerminalCreateResult, ClientError> {
        self.call(ClientMethod::TerminalCreate, params)
    }

    /// Reattaches a reconnectable terminal and returns its rotated recovery lease.
    pub fn terminal_attach(
        &mut self,
        params: TerminalAttachParams,
    ) -> Result<TerminalAttachResult, ClientError> {
        self.call(ClientMethod::TerminalAttach, params)
    }

    /// Writes one bounded UTF-8 input batch to an App Server-owned terminal.
    pub fn terminal_write(&mut self, params: TerminalWriteParams) -> Result<(), ClientError> {
        self.call(ClientMethod::TerminalWrite, params)
    }

    /// Resizes an App Server-owned terminal.
    pub fn terminal_resize(&mut self, params: TerminalResizeParams) -> Result<(), ClientError> {
        self.call(ClientMethod::TerminalResize, params)
    }

    /// Reads terminal output after the caller's last observed output and command sequences.
    pub fn terminal_read(
        &mut self,
        params: TerminalReadParams,
    ) -> Result<TerminalReadResult, ClientError> {
        self.call(ClientMethod::TerminalRead, params)
    }

    /// Closes an App Server-owned terminal.
    pub fn terminal_close(&mut self, params: TerminalCloseParams) -> Result<(), ClientError> {
        self.call(ClientMethod::TerminalClose, params)
    }

    pub fn git_text_diff(&mut self) -> Result<GitTextDiffResult, ClientError> {
        self.call(ClientMethod::GitTextDiff, EmptyParams {})
    }

    pub fn git_status(&mut self) -> Result<GitStatusResult, ClientError> {
        self.call(ClientMethod::GitStatus, EmptyParams {})
    }

    pub fn compute_diff(
        &mut self,
        params: DiffComputeParams,
    ) -> Result<DiffComputeResult, ClientError> {
        self.call(ClientMethod::DiffCompute, params)
    }

    pub fn analyze_syntax(
        &mut self,
        params: SyntaxAnalyzeParams,
    ) -> Result<SyntaxAnalyzeResult, ClientError> {
        self.call(ClientMethod::SyntaxAnalyze, params)
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

    pub fn subscribe_session(
        &mut self,
        params: SessionSubscribeParams,
    ) -> Result<SessionSubscribeResult, ClientError> {
        self.call(ClientMethod::SessionSubscribe, params)
    }

    /// Sends the canonical typed mutation request scoped to one product Session.
    pub fn request_session(
        &mut self,
        params: SessionRequestParams,
    ) -> Result<SessionRequestResult, ClientError> {
        self.call(ClientMethod::SessionRequest, params)
    }

    pub fn unsubscribe_session(
        &mut self,
        params: SessionUnsubscribeParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::SessionUnsubscribe, params)
    }

    pub fn read_session_thread(
        &mut self,
        params: SessionThreadReadParams,
    ) -> Result<SessionThreadReadResult, ClientError> {
        self.call(ClientMethod::SessionThreadRead, params)
    }

    pub fn subscribe_session_thread(
        &mut self,
        params: SessionThreadSubscribeParams,
    ) -> Result<SessionThreadSubscribeResult, ClientError> {
        self.call(ClientMethod::SessionThreadSubscribe, params)
    }

    pub fn unsubscribe_session_thread(
        &mut self,
        params: SessionThreadUnsubscribeParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::SessionThreadUnsubscribe, params)
    }

    pub fn read_config(&mut self) -> Result<ConfigReadResult, ClientError> {
        self.call(ClientMethod::ConfigRead, EmptyParams {})
    }

    pub fn list_connectors(&mut self) -> Result<ConnectorListResult, ClientError> {
        self.call(ClientMethod::ConnectorList, EmptyParams {})
    }

    pub fn connect_connector_api_token(
        &mut self,
        params: ConnectorApiTokenConnectRequest,
    ) -> Result<ConnectorCommandResultDto, ClientError> {
        self.call_secret(ClientMethod::ConnectorApiTokenConnect, params)
    }

    pub fn start_connector_oauth(
        &mut self,
        params: ConnectorOAuthStartParams,
    ) -> Result<ConnectorOAuthStartResult, ClientError> {
        self.call(ClientMethod::ConnectorOAuthStart, params)
    }

    pub fn complete_connector_oauth(
        &mut self,
        params: ConnectorOAuthCompleteRequest,
    ) -> Result<ConnectorCommandResultDto, ClientError> {
        self.call_secret(ClientMethod::ConnectorOAuthComplete, params)
    }

    pub fn cancel_connector_oauth(
        &mut self,
        params: ConnectorOAuthCancelParams,
    ) -> Result<ConnectorCommandResultDto, ClientError> {
        self.call(ClientMethod::ConnectorOAuthCancel, params)
    }

    pub fn start_connector_device_oauth(
        &mut self,
        params: ConnectorDeviceOAuthStartParams,
    ) -> Result<ConnectorDeviceOAuthStartResult, ClientError> {
        self.call(ClientMethod::ConnectorDeviceOAuthStart, params)
    }

    pub fn poll_connector_device_oauth(
        &mut self,
        params: ConnectorDeviceOAuthPollParams,
    ) -> Result<ConnectorDeviceOAuthPollResult, ClientError> {
        self.call(ClientMethod::ConnectorDeviceOAuthPoll, params)
    }

    pub fn cancel_connector_device_oauth(
        &mut self,
        params: ConnectorOAuthCancelParams,
    ) -> Result<ConnectorCommandResultDto, ClientError> {
        self.call(ClientMethod::ConnectorDeviceOAuthCancel, params)
    }

    pub fn refresh_connector_oauth(
        &mut self,
        params: ConnectorOAuthRefreshParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::ConnectorOAuthRefresh, params)
    }

    pub fn revoke_connector_oauth(
        &mut self,
        params: ConnectorDisconnectParams,
    ) -> Result<ConnectorDisconnectResultDto, ClientError> {
        self.call(ClientMethod::ConnectorOAuthRevoke, params)
    }

    pub fn disconnect_connector(
        &mut self,
        params: ConnectorDisconnectParams,
    ) -> Result<ConnectorDisconnectResultDto, ClientError> {
        self.call(ClientMethod::ConnectorDisconnect, params)
    }

    pub fn retry_connector_credential_cleanup(
        &mut self,
        params: ConnectorCredentialCleanupParams,
    ) -> Result<ConnectorCredentialCleanupDto, ClientError> {
        self.call(ClientMethod::ConnectorCredentialCleanupRetry, params)
    }

    pub fn search_marketplace(
        &mut self,
        params: MarketplaceSearchParams,
    ) -> Result<MarketplaceSearchResult, ClientError> {
        self.call(ClientMethod::MarketplaceSearch, params)
    }

    pub fn get_marketplace_package(
        &mut self,
        params: MarketplaceGetParams,
    ) -> Result<MarketplacePackageDetailsDto, ClientError> {
        self.call(ClientMethod::MarketplaceGet, params)
    }

    pub fn download_marketplace_package(
        &mut self,
        params: MarketplaceDownloadParams,
    ) -> Result<MarketplaceArtifactHandleDto, ClientError> {
        self.call(ClientMethod::MarketplaceDownload, params)
    }

    pub fn install_marketplace_package(
        &mut self,
        params: MarketplaceInstallParams,
    ) -> Result<MarketplaceInstalledPackageDto, ClientError> {
        self.call(ClientMethod::MarketplaceInstall, params)
    }

    pub fn update_marketplace_package(
        &mut self,
        params: MarketplaceUpdateParams,
    ) -> Result<MarketplaceInstalledPackageDto, ClientError> {
        self.call(ClientMethod::MarketplaceUpdate, params)
    }

    pub fn uninstall_marketplace_package(
        &mut self,
        params: MarketplaceUninstallParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::MarketplaceUninstall, params)
    }

    pub fn list_installed_marketplace_packages(
        &mut self,
    ) -> Result<MarketplaceListInstalledResult, ClientError> {
        self.call(ClientMethod::MarketplaceListInstalled, EmptyParams {})
    }

    pub fn acquire_marketplace_capability(
        &mut self,
        params: MarketplaceAcquireCapabilityParams,
    ) -> Result<MarketplaceAcquiredCapabilityDto, ClientError> {
        self.call(ClientMethod::MarketplaceAcquireCapability, params)
    }

    pub fn release_marketplace_capability(
        &mut self,
        params: MarketplaceReleaseCapabilityParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::MarketplaceReleaseCapability, params)
    }

    pub fn open_marketplace_resource(
        &mut self,
        params: MarketplaceOpenResourceParams,
    ) -> Result<MarketplaceResourceContentDto, ClientError> {
        self.call(ClientMethod::MarketplaceOpenResource, params)
    }

    pub fn list_plugins(&mut self) -> Result<PluginListResult, ClientError> {
        self.call(ClientMethod::PluginList, EmptyParams {})
    }

    pub fn enable_plugin(
        &mut self,
        params: PluginPackageCommandParams,
    ) -> Result<PluginCommandResultDto, ClientError> {
        self.call(ClientMethod::PluginEnable, params)
    }

    pub fn disable_plugin(
        &mut self,
        params: PluginPackageCommandParams,
    ) -> Result<PluginCommandResultDto, ClientError> {
        self.call(ClientMethod::PluginDisable, params)
    }

    pub fn grant_plugin(
        &mut self,
        params: PluginPackageCommandParams,
    ) -> Result<PluginCommandResultDto, ClientError> {
        self.call(ClientMethod::PluginGrant, params)
    }

    pub fn revoke_plugin_grant(
        &mut self,
        params: PluginPackageCommandParams,
    ) -> Result<PluginCommandResultDto, ClientError> {
        self.call(ClientMethod::PluginRevokeGrant, params)
    }

    pub fn uninstall_plugin(
        &mut self,
        params: PluginPackageCommandParams,
    ) -> Result<PluginCommandResultDto, ClientError> {
        self.call(ClientMethod::PluginUninstall, params)
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

    pub fn open_skill_resource(
        &mut self,
        params: SkillResourceOpenParams,
    ) -> Result<SkillResourceOpenResult, ClientError> {
        self.call(ClientMethod::SkillResourceOpen, params)
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

    pub fn start_attachment_upload(
        &mut self,
        params: AttachmentUploadStartParams,
    ) -> Result<AttachmentUploadStartResult, ClientError> {
        self.call(ClientMethod::AttachmentUploadStart, params)
    }

    pub fn write_attachment_upload(
        &mut self,
        params: AttachmentUploadWriteParams,
    ) -> Result<AttachmentUploadWriteResult, ClientError> {
        self.call(ClientMethod::AttachmentUploadWrite, params)
    }

    pub fn finish_attachment_upload(
        &mut self,
        params: AttachmentUploadFinishParams,
    ) -> Result<AttachmentMaterializeResult, ClientError> {
        self.call(ClientMethod::AttachmentUploadFinish, params)
    }

    pub fn cancel_attachment_upload(
        &mut self,
        params: AttachmentUploadCancelParams,
    ) -> Result<(), ClientError> {
        self.call(ClientMethod::AttachmentUploadCancel, params)
    }

    pub fn import_remote_attachment(
        &mut self,
        params: AttachmentImportRemoteParams,
    ) -> Result<AttachmentMaterializeResult, ClientError> {
        self.call(ClientMethod::AttachmentImportRemote, params)
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

    fn call_secret<P: Serialize, R: for<'a> serde::Deserialize<'a>>(
        &mut self,
        method: ClientMethod,
        params: P,
    ) -> Result<R, ClientError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(request_id),
            method.as_str().into(),
            params,
        );
        let mut encoded_request = Zeroizing::new(
            serde_json::to_string(&request)
                .map_err(|error| ClientError::Protocol(error.to_string()))?,
        );
        let raw_response = self.transport.round_trip(encoded_request.as_str())?;
        encoded_request.zeroize();
        decode_call_response(request_id, &raw_response)
    }
}

fn decode_call_response<R: for<'a> serde::Deserialize<'a>>(
    request_id: u64,
    raw_response: &str,
) -> Result<R, ClientError> {
    let response: JsonRpcResponse<Value, Value> = serde_json::from_str(raw_response)
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
                .to_owned(),
        }),
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
