use crate::protocol::code_index::CloudCodeIndexAuthorizeParams;
use crate::protocol::code_index::CloudCodeIndexDestinationDto;
use crate::protocol::code_index::CloudCodeIndexGrantDto;
use crate::protocol::code_index::CloudCodeIndexPreviewParams;
use crate::protocol::code_index::CloudCodeIndexPreviewResult;
use crate::protocol::code_index::CloudCodeIndexSelectionDto;
use crate::protocol::code_index::CloudCodeIndexStateDto;
use crate::protocol::code_index::CloudCodeIndexStatusResult;
use crate::protocol::code_index::CodeIndexChunkSpanDto;
use crate::protocol::code_index::CodeIndexDeploymentModeDto;
use crate::protocol::code_index::CodeIndexSearchHitDto;
use crate::protocol::code_index::CodeIndexSearchParams;
use crate::protocol::code_index::CodeIndexSearchResult;
use crate::protocol::code_index::CodeIndexStateDto;
use crate::protocol::code_index::CodeIndexStatusResult;
use crate::protocol::code_index::CodeRetrievalDegradationDto;
use crate::protocol::code_index::CodeRetrievalHitDto;
use crate::protocol::code_index::CodeRetrievalOriginDto;
use crate::protocol::code_index::CodeRetrievalParams;
use crate::protocol::code_index::CodeRetrievalResult;
use crate::protocol::code_index::SemanticCodeIndexStateDto;
use crate::protocol::code_index::SemanticCodeIndexStatusDto;
use crate::protocol::collaboration::DocumentCollaborationOpenParams;
use crate::protocol::collaboration::DocumentCollaborationOpenResult;
use crate::protocol::collaboration::DocumentCollaborationPresence;
use crate::protocol::collaboration::DocumentCollaborationPresenceParams;
use crate::protocol::collaboration::DocumentCollaborationPresenceReadParams;
use crate::protocol::collaboration::DocumentCollaborationPresenceSnapshot;
use crate::protocol::collaboration::DocumentCollaborationSnapshot;
use crate::protocol::collaboration::DocumentCollaborationSubmitParams;
use crate::protocol::collaboration::DocumentCollaborationSubmitResult;
use crate::protocol::collaboration::DocumentCollaborationUpdate;
use crate::protocol::common::AgentInteractionCapability;
use crate::protocol::common::{
    BrowserCapability, ClientCapabilities, ClientInfo, CommandId, EmptyParams, ItemId, RequestId,
    SchemaHash, ServerInfo, SessionId, StreamInstanceId, ThreadId, ToolCallId, ToolName, TurnId,
};
use crate::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigChanged, ConfigCommandDispositionDto,
    ConfigCommandResult, ConfigReadResult, ConfigUpdateParams, HookActionDto, HookConfigDto,
    HookEnablementDto, HookEventDto, HookMatcherDto, HookRemoveParams, HookSetEnablementParams,
    HookUpsertParams, LanguageServerConfigDto, LanguageServerConfigureParams,
    LanguageServerModeDto, LanguageServerRemoveParams, McpCredentialBindingDto, McpServerConfigDto,
    McpServerEnablementDto, McpServerRemoveParams, McpServerSetEnablementParams,
    McpServerUpsertParams, McpTransportDto, ModelContextConfigDto, ModelRefDto, PluginRequestDto,
    PluginRequestEnablementDto, PluginRequestRemoveParams, PluginRequestSetEnablementParams,
    PluginRequestUpsertParams, ProviderConfigDto, ProviderConfigureParams, ProviderRemoveParams,
    SemanticCodeIndexAuthorizeParams, SemanticCodeIndexAutomaticContextDto,
    SemanticCodeIndexConfigDto, SemanticCodeIndexConfigureParams, SemanticCodeIndexModelsDto,
    SemanticCodeIndexRevokeParams, SemanticCodeIndexSelectionDto, SkillSourceAddParams,
    SkillSourceConfigDto, SkillSourceEnablementDto, SkillSourceRemoveParams,
    SkillSourceSetEnablementParams, ToolSearchConfigDto, ToolSearchConfigureParams,
    ToolSearchEmbeddingStatusDto, ToolSearchModeDto,
};
use crate::protocol::diff::DiffComputeParams;
use crate::protocol::diff::DiffComputeResult;
use crate::protocol::diff::DiffComputeRowDto;
use crate::protocol::diff::DiffHunkDto;
use crate::protocol::diff::DiffRangeDto;
use crate::protocol::diff::DiffRowKindDto;
use crate::protocol::document::{
    TypstCompileParams, TypstCompileResult, TypstDiagnosticDto, TypstDiagnosticSeverityDto,
    TypstSourceRangeDto,
};
use crate::protocol::error::{AppServerError, AppServerErrorName};
use crate::protocol::extensions::ExtensionCatalogReloadDto;
use crate::protocol::extensions::ExtensionDiagnosticCodeDto;
use crate::protocol::extensions::ExtensionDiagnosticDto;
use crate::protocol::extensions::ExtensionDto;
use crate::protocol::extensions::ExtensionListParams;
use crate::protocol::extensions::ExtensionListResult;
use crate::protocol::extensions::ExtensionResourceOpenParams;
use crate::protocol::extensions::ExtensionResourceOpenResult;
use crate::protocol::extensions::ExtensionSourceKindDto;
use crate::protocol::fs::{
    FsChanged, FsFileType, FsGetMetadataParams, FsGetMetadataResult, FsReadBinaryFileParams,
    FsReadBinaryFileResult, FsReadDirectoryEntry, FsReadDirectoryParams, FsReadDirectoryResult,
    FsReadFileParams, FsReadFileResult, FsWriteFileParams, FsWriteFileResult,
};
use crate::protocol::git::{
    GitBranchDto, GitBranchListResult, GitBranchSwitchParams, GitChangeStatusDto, GitCommitParams,
    GitCommitResult, GitCommitSummaryDto, GitDiffStatisticsDto, GitHeadDto, GitHistoryResult,
    GitOperationResult, GitPathsParams, GitRepositoryChangeDto, GitStatusChanged, GitStatusResult,
    GitSubmoduleStateDto, GitTextDiffDto, GitTextDiffResult, GitUpstreamDto,
};
use crate::protocol::initialize::{InitializeParams, InitializeResult, ServerCapabilities};
use crate::protocol::model::{ModelCatalogEntry, ModelListResult};
use crate::protocol::notification::{SessionUpdateEnvelope, ThreadUpdateEnvelope};
use crate::protocol::resources::{
    ResourceMetadataParams, ResourceMetadataResult, ResourceReadParams, ResourceReadResult,
    ResourceReleaseParams,
};
use crate::protocol::search::{
    WorkspaceSearchCancelParams, WorkspaceSearchCaseSensitivity, WorkspaceSearchMatch,
    WorkspaceSearchMatchRange, WorkspaceSearchPatternKind, WorkspaceSearchReadParams,
    WorkspaceSearchReadResult, WorkspaceSearchStartParams, WorkspaceSearchStartResult,
};
use crate::protocol::session::{
    SessionCreateParams, SessionListResult, SessionReadParams, SessionRequest,
    SessionRequestParams, SessionRequestResult, SessionResult, SessionSubscribeParams,
    SessionSubscribeResult, SessionThreadProjection, SessionThreadReadParams,
    SessionThreadReadResult, SessionThreadResult, SessionThreadSubscribeParams,
    SessionThreadSubscribeResult, SessionThreadUnsubscribeParams, SessionUnsubscribeParams,
    ThreadHistoryBoundary, ThreadSnapshotHistory,
};
use crate::protocol::skills::{
    SkillCatalogReloadDto, SkillCompatibilityDto, SkillDiagnosticCodeDto, SkillDiagnosticDto,
    SkillDto, SkillEnablementDto, SkillListParams, SkillListResult, SkillSetEnablementParams,
    SkillSourceKindDto, SkillsChanged,
};
use crate::protocol::slash_commands::{SlashCommandArgumentModeDto, SlashCommandDefinition};
use crate::protocol::syntax::SyntaxAnalyzeParams;
use crate::protocol::syntax::SyntaxAnalyzeResult;
use crate::protocol::syntax::SyntaxDiagnosticDto;
use crate::protocol::syntax::SyntaxDiagnosticKindDto;
use crate::protocol::syntax::SyntaxFoldingRangeDto;
use crate::protocol::syntax::SyntaxLanguageDto;
use crate::protocol::syntax::SyntaxPositionDto;
use crate::protocol::syntax::SyntaxRangeDto;
use crate::protocol::syntax::SyntaxSymbolDto;
use crate::protocol::syntax::SyntaxSymbolKindDto;
use crate::protocol::syntax::SyntaxTokenDto;
use crate::protocol::syntax::SyntaxTokenKindDto;
use crate::protocol::terminal::{
    TerminalCloseParams, TerminalCommandStatus, TerminalCommandStatusEvent, TerminalCreateParams,
    TerminalCreateResult, TerminalOutputChunk, TerminalProfile, TerminalProfileListResult,
    TerminalProfileSelection, TerminalReadParams, TerminalReadResult, TerminalResizeParams,
    TerminalWriteParams,
};
use crate::protocol::turn::{
    InputItem, TurnInteractionResolveResult, TurnInterruptResult, TurnStartResult,
};
use crate::protocol::workspace::{WorkspaceSwitchParams, WorkspaceSwitchResult};
use schemars::JsonSchema;
use ts_rs::{Config, TS};
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalDecision,
    ActionApprovalRequest, ActionApprovalResponse, AgentContextContent, AgentContextMode,
    AgentContextSeed, AgentContextSource, AgentInteractionKind, AgentJoin, AgentJoinId,
    AgentJoinPolicy, AgentJoinStatus, AgentMaterializedContext, AgentMessage, AgentMessageContent,
    AgentMessageId, AgentMessageProvenance, AgentRequest, AgentResponse, AgentRoleSnapshot,
    ContentDigest, ContextCheckpoint, ContextCheckpointId, ContextCheckpointVerification,
    ContextSeedDigest, ContextSourceDigest, ContextSourceRange, DelegatedCapabilityScope,
    DelegatedPolicyCeiling, DelegatedTask, DelegationArtifactRef, DelegationId, DelegationResult,
    DelegationResultDigest, DelegationResultStatus, DynamicToolCall, DynamicToolOutput,
    DynamicToolResponse, ForkedAgentContext, FrozenSkillActivation, InteractionCancelReason,
    InteractionDeadline, ItemDelta, PendingInteraction, PlanStep, PlanStepStatus, PlanUpdate,
    ProcessExecutionOutput, ProcessExitStatus, RequestUserInput, RequestUserInputResponse,
    SandboxDenialOutput, Session, SessionEvent, SessionStatus, SessionThread, SessionThreadStatus,
    SessionUpdate, SkillActivationReason, SkillId, SkillName, SkillRef, SkillSourceId,
    SkillVersionSelector, StableTurnError, StableTurnErrorCode, StreamCursor, Thread, ThreadEvent,
    ThreadItem, ThreadOrigin, ThreadSequenceRange, ThreadStatus, ThreadUpdate,
    ToolExecutionAuthority, ToolOutputStream, ToolReplaySafety, Turn, TurnInteraction, TurnStatus,
    UserInputAnswer, UserInputOption, UserInputQuestion,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerializationScopeDefinition {
    None,
    GlobalExclusive,
    GlobalSharedRead,
    SessionExclusive,
    SessionSharedRead,
    ResourceExclusive,
}

#[derive(Clone, Copy)]
pub struct ClientMethodDefinition {
    pub kind: ClientMethod,
    pub method: &'static str,
    pub serialization: SerializationScopeDefinition,
    params_type: fn() -> String,
    result_type: fn() -> String,
}

impl ClientMethodDefinition {
    pub fn params_type(&self) -> String {
        (self.params_type)()
    }

    pub fn result_type(&self) -> String {
        (self.result_type)()
    }
}

#[derive(Clone, Copy)]
pub struct ServerNotificationDefinition {
    pub kind: ServerNotificationMethod,
    pub method: &'static str,
    params_type: fn() -> String,
}

impl ServerNotificationDefinition {
    pub fn params_type(&self) -> String {
        (self.params_type)()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TypeScriptBinding {
    declaration: fn() -> String,
}

impl TypeScriptBinding {
    pub(crate) fn declaration(&self) -> String {
        (self.declaration)()
    }
}

fn type_name<T: TS>() -> String {
    T::name(&Config::default())
}

fn declaration<T: TS>() -> String {
    T::decl(&Config::default())
}

macro_rules! client_methods {
    (
        $(
            $variant:ident => $method:literal {
                params: $params:ty,
                response: $response:ty,
                serialization: $serialization:ident,
            }
        ),+ $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum ClientMethod {
            $($variant,)+
        }

        impl ClientMethod {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $method,)+
                }
            }
        }

        pub fn client_method(method: &str) -> Option<ClientMethod> {
            match method {
                $($method => Some(ClientMethod::$variant),)+
                _ => None,
            }
        }

        pub const CLIENT_METHODS: &[ClientMethodDefinition] = &[
            $(
                ClientMethodDefinition {
                    kind: ClientMethod::$variant,
                    method: $method,
                    serialization: SerializationScopeDefinition::$serialization,
                    params_type: type_name::<$params>,
                    result_type: type_name::<$response>,
                },
            )+
        ];

        #[allow(dead_code)]
        #[derive(JsonSchema)]
        #[serde(tag = "method", content = "params")]
        pub(crate) enum ClientRequestSchema {
            $(
                #[serde(rename = $method)]
                $variant($params),
            )+
        }

        #[allow(dead_code)]
        #[derive(JsonSchema)]
        #[serde(tag = "method", content = "result")]
        pub(crate) enum ClientResultSchema {
            $(
                #[serde(rename = $method)]
                $variant(Box<$response>),
            )+
        }
    };
}

client_methods! {
    Initialize => "initialize" {
        params: InitializeParams,
        response: InitializeResult,
        serialization: GlobalExclusive,
    },
    WorkspaceSwitch => "workspace/switch" {
        params: WorkspaceSwitchParams,
        response: WorkspaceSwitchResult,
        serialization: GlobalExclusive,
    },
    DocumentCollaborationOpen => "document/collaboration/open" {
        params: DocumentCollaborationOpenParams,
        response: DocumentCollaborationOpenResult,
        serialization: GlobalExclusive,
    },
    DocumentCollaborationSubmit => "document/collaboration/submit" {
        params: DocumentCollaborationSubmitParams,
        response: DocumentCollaborationSubmitResult,
        serialization: GlobalExclusive,
    },
    DocumentCollaborationPresencePublish => "document/collaboration/presence/publish" {
        params: DocumentCollaborationPresenceParams,
        response: DocumentCollaborationPresenceSnapshot,
        serialization: GlobalExclusive,
    },
    DocumentCollaborationPresenceRead => "document/collaboration/presence/read" {
        params: DocumentCollaborationPresenceReadParams,
        response: DocumentCollaborationPresenceSnapshot,
        serialization: GlobalSharedRead,
    },
    SessionCreate => "session/create" {
        params: SessionCreateParams,
        response: SessionResult,
        serialization: GlobalExclusive,
    },
    SessionRead => "session/read" {
        params: SessionReadParams,
        response: SessionResult,
        serialization: SessionSharedRead,
    },
    SessionList => "session/list" {
        params: EmptyParams,
        response: SessionListResult,
        serialization: GlobalSharedRead,
    },
    SessionSubscribe => "session/subscribe" {
        params: SessionSubscribeParams,
        response: SessionSubscribeResult,
        serialization: SessionSharedRead,
    },
    SessionRequest => "session/request" {
        params: SessionRequestParams,
        response: SessionRequestResult,
        serialization: SessionExclusive,
    },
    SessionUnsubscribe => "session/unsubscribe" {
        params: SessionUnsubscribeParams,
        response: (),
        serialization: None,
    },
    SessionThreadRead => "session/thread/read" {
        params: SessionThreadReadParams,
        response: SessionThreadReadResult,
        serialization: SessionSharedRead,
    },
    SessionThreadSubscribe => "session/thread/subscribe" {
        params: SessionThreadSubscribeParams,
        response: SessionThreadSubscribeResult,
        serialization: SessionSharedRead,
    },
    SessionThreadUnsubscribe => "session/thread/unsubscribe" {
        params: SessionThreadUnsubscribeParams,
        response: (),
        serialization: None,
    },
    ConfigRead => "config/read" {
        params: EmptyParams,
        response: ConfigReadResult,
        serialization: GlobalSharedRead,
    },
    ModelList => "model/list" {
        params: EmptyParams,
        response: ModelListResult,
        serialization: GlobalSharedRead,
    },
    ConfigUpdate => "config/update" {
        params: ConfigUpdateParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    ToolSearchConfigure => "toolSearch/configure" {
        params: ToolSearchConfigureParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SemanticCodeIndexConfigure => "workspace/codeIndex/semantic/configure" {
        params: SemanticCodeIndexConfigureParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SemanticCodeIndexAuthorize => "workspace/codeIndex/semantic/authorize" {
        params: SemanticCodeIndexAuthorizeParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SemanticCodeIndexRevoke => "workspace/codeIndex/semantic/revoke" {
        params: SemanticCodeIndexRevokeParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    LanguageServerConfigure => "languageServer/configure" {
        params: LanguageServerConfigureParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    LanguageServerRemove => "languageServer/remove" {
        params: LanguageServerRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    ProviderConfigure => "provider/configure" {
        params: ProviderConfigureParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    ProviderRemove => "provider/remove" {
        params: ProviderRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    McpServerUpsert => "mcp/server/upsert" {
        params: McpServerUpsertParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    McpServerRemove => "mcp/server/remove" {
        params: McpServerRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    McpServerSetEnablement => "mcp/server/enablement/set" {
        params: McpServerSetEnablementParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SkillSourceAdd => "skill/source/add" {
        params: SkillSourceAddParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SkillSourceRemove => "skill/source/remove" {
        params: SkillSourceRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SkillSourceSetEnablement => "skill/source/enablement/set" {
        params: SkillSourceSetEnablementParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    PluginRequestUpsert => "plugin/request/upsert" {
        params: PluginRequestUpsertParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    PluginRequestRemove => "plugin/request/remove" {
        params: PluginRequestRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    PluginRequestSetEnablement => "plugin/request/enablement/set" {
        params: PluginRequestSetEnablementParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    HookUpsert => "hook/upsert" {
        params: HookUpsertParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    HookRemove => "hook/remove" {
        params: HookRemoveParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    HookSetEnablement => "hook/enablement/set" {
        params: HookSetEnablementParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    SkillList => "skills/list" {
        params: SkillListParams,
        response: SkillListResult,
        serialization: GlobalSharedRead,
    },
    SkillSetEnablement => "skill/enablement/set" {
        params: SkillSetEnablementParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    ExtensionList => "extensions/list" {
        params: ExtensionListParams,
        response: ExtensionListResult,
        serialization: GlobalSharedRead,
    },
    ExtensionResourceOpen => "extensions/resource/open" {
        params: ExtensionResourceOpenParams,
        response: ExtensionResourceOpenResult,
        serialization: ResourceExclusive,
    },
    TypstCompile => "document/typst/compile" {
        params: TypstCompileParams,
        response: TypstCompileResult,
        serialization: GlobalExclusive,
    },
    ResourceMetadata => "resource/metadata" {
        params: ResourceMetadataParams,
        response: ResourceMetadataResult,
        serialization: ResourceExclusive,
    },
    ResourceRead => "resource/read" {
        params: ResourceReadParams,
        response: ResourceReadResult,
        serialization: ResourceExclusive,
    },
    ResourceRelease => "resource/release" {
        params: ResourceReleaseParams,
        response: (),
        serialization: ResourceExclusive,
    },
    FsGetMetadata => "fs/getMetadata" {
        params: FsGetMetadataParams,
        response: FsGetMetadataResult,
        serialization: GlobalSharedRead,
    },
    FsReadDirectory => "fs/readDirectory" {
        params: FsReadDirectoryParams,
        response: FsReadDirectoryResult,
        serialization: GlobalSharedRead,
    },
    FsReadFile => "fs/readFile" {
        params: FsReadFileParams,
        response: FsReadFileResult,
        serialization: GlobalSharedRead,
    },
    FsReadBinaryFile => "fs/readBinaryFile" {
        params: FsReadBinaryFileParams,
        response: FsReadBinaryFileResult,
        serialization: GlobalSharedRead,
    },
    DiffCompute => "diff/compute" {
        params: DiffComputeParams,
        response: DiffComputeResult,
        serialization: GlobalSharedRead,
    },
    SyntaxAnalyze => "syntax/analyze" {
        params: SyntaxAnalyzeParams,
        response: SyntaxAnalyzeResult,
        serialization: GlobalSharedRead,
    },
    FsWriteFile => "fs/writeFile" {
        params: FsWriteFileParams,
        response: FsWriteFileResult,
        serialization: GlobalExclusive,
    },
    GitStatus => "git/status" {
        params: EmptyParams,
        response: GitStatusResult,
        serialization: GlobalSharedRead,
    },
    GitTextDiff => "git/textDiff" {
        params: EmptyParams,
        response: GitTextDiffResult,
        serialization: GlobalSharedRead,
    },
    GitBranchList => "git/branch/list" {
        params: EmptyParams,
        response: GitBranchListResult,
        serialization: GlobalSharedRead,
    },
    GitHistory => "git/history" {
        params: EmptyParams,
        response: GitHistoryResult,
        serialization: GlobalSharedRead,
    },
    GitBranchSwitch => "git/branch/switch" {
        params: GitBranchSwitchParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitStage => "git/stage" {
        params: GitPathsParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitUnstage => "git/unstage" {
        params: GitPathsParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitDiscardWorktree => "git/discardWorktree" {
        params: GitPathsParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitCommit => "git/commit" {
        params: GitCommitParams,
        response: GitCommitResult,
        serialization: GlobalExclusive,
    },
    GitFetch => "git/fetch" {
        params: EmptyParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitPull => "git/pull" {
        params: EmptyParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitPush => "git/push" {
        params: EmptyParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    WorkspaceSearchStart => "workspace/search/start" {
        params: WorkspaceSearchStartParams,
        response: WorkspaceSearchStartResult,
        serialization: None,
    },
    WorkspaceSearchRead => "workspace/search/read" {
        params: WorkspaceSearchReadParams,
        response: WorkspaceSearchReadResult,
        serialization: None,
    },
    WorkspaceSearchCancel => "workspace/search/cancel" {
        params: WorkspaceSearchCancelParams,
        response: (),
        serialization: None,
    },
    CodeIndexStatus => "workspace/codeIndex/status" {
        params: EmptyParams,
        response: CodeIndexStatusResult,
        serialization: GlobalSharedRead,
    },
    CodeIndexSearch => "workspace/codeIndex/search" {
        params: CodeIndexSearchParams,
        response: CodeIndexSearchResult,
        serialization: GlobalSharedRead,
    },
    CodeIndexRetrieve => "workspace/codeIndex/retrieve" {
        params: CodeRetrievalParams,
        response: CodeRetrievalResult,
        serialization: GlobalSharedRead,
    },
    CodeIndexRebuild => "workspace/codeIndex/rebuild" {
        params: EmptyParams,
        response: CodeIndexStatusResult,
        serialization: GlobalExclusive,
    },
    SemanticCodeIndexCancel => "workspace/codeIndex/semantic/cancel" {
        params: EmptyParams,
        response: CodeIndexStatusResult,
        serialization: None,
    },
    SemanticCodeIndexRetry => "workspace/codeIndex/semantic/retry" {
        params: EmptyParams,
        response: CodeIndexStatusResult,
        serialization: None,
    },
    CloudCodeIndexStatus => "workspace/codeIndex/cloud/status" {
        params: EmptyParams,
        response: CloudCodeIndexStatusResult,
        serialization: GlobalSharedRead,
    },
    CloudCodeIndexPreview => "workspace/codeIndex/cloud/preview" {
        params: CloudCodeIndexPreviewParams,
        response: CloudCodeIndexPreviewResult,
        serialization: GlobalSharedRead,
    },
    CloudCodeIndexAuthorize => "workspace/codeIndex/cloud/authorize" {
        params: CloudCodeIndexAuthorizeParams,
        response: CloudCodeIndexStatusResult,
        serialization: GlobalExclusive,
    },
    CloudCodeIndexSync => "workspace/codeIndex/cloud/sync" {
        params: EmptyParams,
        response: CloudCodeIndexStatusResult,
        serialization: GlobalExclusive,
    },
    CloudCodeIndexRevoke => "workspace/codeIndex/cloud/revoke" {
        params: EmptyParams,
        response: CloudCodeIndexStatusResult,
        serialization: GlobalExclusive,
    },
    TerminalProfileList => "terminal/profile/list" {
        params: EmptyParams,
        response: TerminalProfileListResult,
        serialization: GlobalSharedRead,
    },
    TerminalCreate => "terminal/create" {
        params: TerminalCreateParams,
        response: TerminalCreateResult,
        serialization: None,
    },
    TerminalWrite => "terminal/write" {
        params: TerminalWriteParams,
        response: (),
        serialization: None,
    },
    TerminalResize => "terminal/resize" {
        params: TerminalResizeParams,
        response: (),
        serialization: None,
    },
    TerminalRead => "terminal/read" {
        params: TerminalReadParams,
        response: TerminalReadResult,
        serialization: None,
    },
    TerminalClose => "terminal/close" {
        params: TerminalCloseParams,
        response: (),
        serialization: None,
    },
}

macro_rules! server_notifications {
    (
        $(
            $variant:ident => $method:literal {
                params: $params:ty,
            }
        ),+ $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum ServerNotificationMethod {
            $($variant,)+
        }

        impl ServerNotificationMethod {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $method,)+
                }
            }
        }

        pub fn server_notification_method(method: &str) -> Option<ServerNotificationMethod> {
            match method {
                $($method => Some(ServerNotificationMethod::$variant),)+
                _ => None,
            }
        }

        pub const SERVER_NOTIFICATIONS: &[ServerNotificationDefinition] = &[
            $(
                ServerNotificationDefinition {
                    kind: ServerNotificationMethod::$variant,
                    method: $method,
                    params_type: type_name::<$params>,
                },
            )+
        ];

        #[allow(dead_code, clippy::large_enum_variant)]
        #[derive(JsonSchema)]
        #[serde(tag = "method", content = "params")]
        pub(crate) enum ServerNotificationSchema {
            $(
                #[serde(rename = $method)]
                $variant($params),
            )+
        }
    };
}

server_notifications! {
    AgentRequest => "agent/request" {
        params: AgentRequestEnvelope,
    },
    DocumentCollaborationUpdate => "document/collaboration/update" {
        params: DocumentCollaborationUpdate,
    },
    DocumentCollaborationPresence => "document/collaboration/presence" {
        params: DocumentCollaborationPresenceSnapshot,
    },
    SessionUpdate => "session/update" {
        params: SessionUpdateEnvelope,
    },
    SessionThreadUpdate => "session/thread/update" {
        params: ThreadUpdateEnvelope,
    },
    ConfigChanged => "config/changed" {
        params: ConfigChanged,
    },
    SkillsChanged => "skills/changed" {
        params: SkillsChanged,
    },
    GitStatusChanged => "git/statusChanged" {
        params: GitStatusChanged,
    },
    FsChanged => "fs/changed" {
        params: FsChanged,
    },
}

macro_rules! typescript_bindings {
    ($($type:ty),+ $(,)?) => {
        pub(crate) const TYPESCRIPT_BINDINGS: &[TypeScriptBinding] = &[
            $(
                TypeScriptBinding {
                    declaration: declaration::<$type>,
                },
            )+
        ];
    };
}

typescript_bindings! {
    ThreadId,
    SessionId,
    CommandId,
    RequestId,
    StreamInstanceId,
    ItemId,
    ToolCallId,
    ToolName,
    TurnId,
    DelegationId,
    AgentJoinId,
    AgentMessageId,
    SchemaHash,
    ClientInfo,
    AgentInteractionCapability,
    BrowserCapability,
    ClientCapabilities,
    ServerInfo,
    DocumentCollaborationOpenParams,
    DocumentCollaborationSnapshot,
    DocumentCollaborationOpenResult,
    DocumentCollaborationPresence,
    DocumentCollaborationPresenceParams,
    DocumentCollaborationPresenceReadParams,
    DocumentCollaborationPresenceSnapshot,
    DocumentCollaborationUpdate,
    DocumentCollaborationSubmitParams,
    DocumentCollaborationSubmitResult,
    ModelRefDto,
    SemanticCodeIndexModelsDto,
    SemanticCodeIndexSelectionDto,
    SemanticCodeIndexAutomaticContextDto,
    SemanticCodeIndexConfigDto,
    ApprovalReviewModelSelectionDto,
    ModelContextConfigDto,
    ProviderConfigDto,
    McpCredentialBindingDto,
    McpServerEnablementDto,
    McpTransportDto,
    McpServerConfigDto,
    SkillSourceEnablementDto,
    SkillSourceConfigDto,
    PluginRequestEnablementDto,
    PluginRequestDto,
    HookEventDto,
    HookEnablementDto,
    HookMatcherDto,
    HookActionDto,
    HookConfigDto,
    LanguageServerModeDto,
    LanguageServerConfigDto,
    ConfigReadResult,
    ConfigChanged,
    ConfigCommandDispositionDto,
    ConfigCommandResult,
    ConfigUpdateParams,
    ToolSearchModeDto,
    ToolSearchEmbeddingStatusDto,
    ToolSearchConfigDto,
    ToolSearchConfigureParams,
    SemanticCodeIndexConfigureParams,
    SemanticCodeIndexAuthorizeParams,
    SemanticCodeIndexRevokeParams,
    LanguageServerConfigureParams,
    LanguageServerRemoveParams,
    ProviderConfigureParams,
    ProviderRemoveParams,
    McpServerUpsertParams,
    McpServerRemoveParams,
    McpServerSetEnablementParams,
    SkillSourceAddParams,
    SkillSourceRemoveParams,
    SkillSourceSetEnablementParams,
    PluginRequestUpsertParams,
    PluginRequestRemoveParams,
    PluginRequestSetEnablementParams,
    HookUpsertParams,
    HookRemoveParams,
    HookSetEnablementParams,
    SkillName,
    SkillSourceId,
    SkillId,
    ContentDigest,
    DelegatedTask,
    AgentRoleSnapshot,
    AgentContextSource,
    AgentContextContent,
    AgentMaterializedContext,
    ForkedAgentContext,
    AgentContextMode,
    DelegatedPolicyCeiling,
    DelegatedCapabilityScope,
    ContextSeedDigest,
    AgentContextSeed,
    ThreadSequenceRange,
    DelegationResultStatus,
    DelegationArtifactRef,
    DelegationResultDigest,
    DelegationResult,
    AgentMessageProvenance,
    AgentMessageContent,
    AgentMessage,
    AgentJoinPolicy,
    AgentJoinStatus,
    AgentJoin,
    SkillVersionSelector,
    SkillRef,
    SkillActivationReason,
    FrozenSkillActivation,
    SkillCatalogReloadDto,
    SkillEnablementDto,
    SkillSourceKindDto,
    SkillCompatibilityDto,
    SkillDto,
    SkillDiagnosticCodeDto,
    SkillDiagnosticDto,
    SkillListParams,
    SkillListResult,
    SkillSetEnablementParams,
    SkillsChanged,
    ExtensionCatalogReloadDto,
    ExtensionSourceKindDto,
    ExtensionDiagnosticCodeDto,
    ExtensionDto,
    ExtensionDiagnosticDto,
    ExtensionListParams,
    ExtensionListResult,
    ExtensionResourceOpenParams,
    ExtensionResourceOpenResult,
    SlashCommandArgumentModeDto,
    SlashCommandDefinition,
    ServerCapabilities,
    InitializeParams,
    InitializeResult,
    WorkspaceSwitchParams,
    WorkspaceSwitchResult,
    SessionStatus,
    ThreadOrigin,
    SessionThreadStatus,
    SessionThread,
    Session,
    SessionEvent,
    SessionUpdate,
    SessionUpdateEnvelope,
    SessionCreateParams,
    SessionReadParams,
    SessionSubscribeParams,
    SessionUnsubscribeParams,
    SessionRequest,
    SessionRequestParams,
    SessionRequestResult,
    SessionThreadReadParams,
    SessionThreadReadResult,
    SessionThreadSubscribeParams,
    SessionThreadSubscribeResult,
    SessionThreadUnsubscribeParams,
    SessionResult,
    SessionListResult,
    SessionSubscribeResult,
    SessionThreadProjection,
    SessionThreadResult,
    ThreadSnapshotHistory,
    ThreadHistoryBoundary,
    ModelCatalogEntry,
    ModelListResult,
    StableTurnErrorCode,
    StableTurnError,
    ThreadStatus,
    TurnStatus,
    ActionApprovalCapabilityKind,
    ActionApprovalCapability,
    ActionApprovalRequest,
    ActionApprovalDecision,
    ActionApprovalResponse,
    AgentInteractionKind,
    AgentRequest,
    AgentRequestEnvelope,
    AgentResponse,
    TurnInteraction,
    PendingInteraction,
    InteractionDeadline,
    InteractionCancelReason,
    RequestUserInput,
    RequestUserInputResponse,
    UserInputQuestion,
    UserInputOption,
    UserInputAnswer,
    DynamicToolCall,
    DynamicToolResponse,
    DynamicToolOutput,
    ThreadItem,
    Turn,
    Thread,
    ToolExecutionAuthority,
    ToolOutputStream,
    ProcessExitStatus,
    ProcessExecutionOutput,
    ToolReplaySafety,
    SandboxDenialOutput,
    ContextCheckpointId,
    ContextSourceRange,
    ContextSourceDigest,
    ContextCheckpointVerification,
    ContextCheckpoint,
    ThreadEvent,
    PlanStepStatus,
    PlanStep,
    PlanUpdate,
    StreamCursor,
    ItemDelta,
    ThreadUpdate,
    ThreadUpdateEnvelope,
    InputItem,
    TurnStartResult,
    TurnInterruptResult,
    TurnInteractionResolveResult,
    TypstCompileParams,
    TypstCompileResult,
    TypstDiagnosticDto,
    TypstDiagnosticSeverityDto,
    TypstSourceRangeDto,
    ResourceMetadataParams,
    ResourceMetadataResult,
    ResourceReadParams,
    ResourceReadResult,
    ResourceReleaseParams,
    FsFileType,
    FsGetMetadataParams,
    FsGetMetadataResult,
    FsReadDirectoryParams,
    FsReadDirectoryEntry,
    FsReadDirectoryResult,
    FsReadBinaryFileParams,
    FsReadBinaryFileResult,
    FsReadFileParams,
    FsReadFileResult,
    DiffComputeParams,
    DiffRowKindDto,
    DiffRangeDto,
    DiffComputeRowDto,
    DiffHunkDto,
    DiffComputeResult,
    SyntaxLanguageDto,
    SyntaxPositionDto,
    SyntaxRangeDto,
    SyntaxTokenKindDto,
    SyntaxTokenDto,
    SyntaxFoldingRangeDto,
    SyntaxSymbolKindDto,
    SyntaxSymbolDto,
    SyntaxDiagnosticKindDto,
    SyntaxDiagnosticDto,
    SyntaxAnalyzeParams,
    SyntaxAnalyzeResult,
    FsWriteFileParams,
    FsWriteFileResult,
    FsChanged,
    GitChangeStatusDto,
    GitUpstreamDto,
    GitHeadDto,
    GitSubmoduleStateDto,
    GitRepositoryChangeDto,
    GitStatusResult,
    GitStatusChanged,
    GitBranchDto,
    GitBranchListResult,
    GitCommitSummaryDto,
    GitHistoryResult,
    GitBranchSwitchParams,
    GitTextDiffDto,
    GitDiffStatisticsDto,
    GitTextDiffResult,
    GitPathsParams,
    GitCommitParams,
    GitOperationResult,
    GitCommitResult,
    WorkspaceSearchPatternKind,
    WorkspaceSearchCaseSensitivity,
    WorkspaceSearchStartParams,
    WorkspaceSearchStartResult,
    WorkspaceSearchReadParams,
    WorkspaceSearchMatchRange,
    WorkspaceSearchMatch,
    WorkspaceSearchReadResult,
    WorkspaceSearchCancelParams,
    CodeIndexStateDto,
    CodeIndexStatusResult,
    SemanticCodeIndexStateDto,
    SemanticCodeIndexStatusDto,
    CodeIndexSearchParams,
    CodeIndexChunkSpanDto,
    CodeIndexSearchHitDto,
    CodeIndexSearchResult,
    CodeRetrievalParams,
    CodeRetrievalOriginDto,
    CodeRetrievalDegradationDto,
    CodeRetrievalHitDto,
    CodeRetrievalResult,
    CodeIndexDeploymentModeDto,
    CloudCodeIndexStateDto,
    CloudCodeIndexSelectionDto,
    CloudCodeIndexDestinationDto,
    CloudCodeIndexGrantDto,
    CloudCodeIndexPreviewParams,
    CloudCodeIndexPreviewResult,
    CloudCodeIndexAuthorizeParams,
    CloudCodeIndexStatusResult,
    TerminalProfile,
    TerminalProfileListResult,
    TerminalProfileSelection,
    TerminalCreateParams,
    TerminalCreateResult,
    TerminalWriteParams,
    TerminalResizeParams,
    TerminalReadParams,
    TerminalOutputChunk,
    TerminalCommandStatus,
    TerminalCommandStatusEvent,
    TerminalReadResult,
    TerminalCloseParams,
    AppServerErrorName,
    AppServerError,
}
