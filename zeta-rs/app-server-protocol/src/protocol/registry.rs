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
    McpServerUpsertParams, McpTransportDto, ModelRefDto, PluginRequestDto,
    PluginRequestEnablementDto, PluginRequestRemoveParams, PluginRequestSetEnablementParams,
    PluginRequestUpsertParams, ProviderConfigDto, ProviderConfigureParams, ProviderRemoveParams,
    SkillSourceAddParams, SkillSourceConfigDto, SkillSourceEnablementDto, SkillSourceRemoveParams,
    SkillSourceSetEnablementParams,
};
use crate::protocol::document::{
    TypstCompileParams, TypstCompileResult, TypstDiagnosticDto, TypstDiagnosticSeverityDto,
    TypstSourceRangeDto,
};
use crate::protocol::error::{AppServerError, AppServerErrorName};
use crate::protocol::fs::{
    FsChanged, FsFileType, FsGetMetadataParams, FsGetMetadataResult, FsReadDirectoryEntry,
    FsReadDirectoryParams, FsReadDirectoryResult, FsReadFileParams, FsReadFileResult,
    FsWriteFileParams, FsWriteFileResult,
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
    SessionCommandParams, SessionCreateParams, SessionListResult, SessionModelSetParams,
    SessionReadParams, SessionResult, SessionSubscribeParams, SessionSubscribeResult,
    SessionThreadArchiveParams, SessionThreadCreateParams, SessionThreadForkParams,
    SessionThreadResult, SessionThreadRewindParams, SessionUnsubscribeParams,
};
use crate::protocol::skills::{
    SkillCatalogReloadDto, SkillCompatibilityDto, SkillDiagnosticCodeDto, SkillDiagnosticDto,
    SkillDto, SkillEnablementDto, SkillListParams, SkillListResult, SkillSetEnablementParams,
    SkillSourceKindDto, SkillsChanged,
};
use crate::protocol::slash_commands::{SlashCommandArgumentModeDto, SlashCommandDefinition};
use crate::protocol::terminal::{
    TerminalCloseParams, TerminalCommandStatus, TerminalCommandStatusEvent, TerminalCreateParams,
    TerminalCreateResult, TerminalOutputChunk, TerminalProfile, TerminalProfileListResult,
    TerminalProfileSelection, TerminalReadParams, TerminalReadResult, TerminalResizeParams,
    TerminalWriteParams,
};
use crate::protocol::thread::{
    ThreadReadParams, ThreadReadResult, ThreadSubscribeParams, ThreadSubscribeResult,
    ThreadUnsubscribeParams,
};
use crate::protocol::turn::{
    InputItem, ShellTurnStartParams, TurnInteractionResolveParams, TurnInteractionResolveResult,
    TurnInterruptParams, TurnInterruptResult, TurnStartParams, TurnStartResult,
};
use crate::protocol::workspace::{WorkspaceSwitchParams, WorkspaceSwitchResult};
use schemars::JsonSchema;
use ts_rs::{Config, TS};
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalDecision,
    ActionApprovalRequest, ActionApprovalResponse, AgentInteractionKind, AgentRequest,
    AgentResponse, DynamicToolCall, DynamicToolOutput, DynamicToolResponse,
    InteractionCancelReason, InteractionDeadline, ItemDelta, PendingInteraction, PlanStep,
    PlanStepStatus, PlanUpdate, ProcessExecutionOutput, ProcessExitStatus, RequestUserInput,
    RequestUserInputResponse, SandboxDenialOutput, Session, SessionEvent, SessionStatus,
    SessionThread, SessionThreadStatus, SessionUpdate, SkillId, SkillName, SkillSourceId,
    StableTurnError, StableTurnErrorCode, StreamCursor, Thread, ThreadEvent, ThreadItem,
    ThreadOrigin, ThreadStatus, ThreadUpdate, ToolExecutionAuthority, ToolOutputStream,
    ToolReplaySafety, Turn, TurnInteraction, TurnStatus, UserInputAnswer, UserInputOption,
    UserInputQuestion,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerializationScopeDefinition {
    None,
    GlobalExclusive,
    GlobalSharedRead,
    SessionExclusive,
    SessionSharedRead,
    ThreadExclusive,
    ThreadSharedRead,
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
                $variant($response),
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
    SessionUnsubscribe => "session/unsubscribe" {
        params: SessionUnsubscribeParams,
        response: (),
        serialization: None,
    },
    SessionThreadCreate => "session/thread/create" {
        params: SessionThreadCreateParams,
        response: SessionThreadResult,
        serialization: SessionExclusive,
    },
    SessionThreadFork => "session/thread/fork" {
        params: SessionThreadForkParams,
        response: SessionThreadResult,
        serialization: SessionExclusive,
    },
    SessionThreadRewind => "session/thread/rewind" {
        params: SessionThreadRewindParams,
        response: SessionThreadResult,
        serialization: SessionExclusive,
    },
    SessionThreadArchive => "session/thread/archive" {
        params: SessionThreadArchiveParams,
        response: SessionResult,
        serialization: SessionExclusive,
    },
    SessionComplete => "session/complete" {
        params: SessionCommandParams,
        response: SessionResult,
        serialization: SessionExclusive,
    },
    SessionArchive => "session/archive" {
        params: SessionCommandParams,
        response: SessionResult,
        serialization: SessionExclusive,
    },
    SessionStop => "session/stop" {
        params: SessionCommandParams,
        response: SessionResult,
        serialization: SessionExclusive,
    },
    SessionModelSet => "session/model/set" {
        params: SessionModelSetParams,
        response: SessionResult,
        serialization: SessionExclusive,
    },
    ThreadRead => "thread/read" {
        params: ThreadReadParams,
        response: ThreadReadResult,
        serialization: ThreadSharedRead,
    },
    ThreadSubscribe => "thread/subscribe" {
        params: ThreadSubscribeParams,
        response: ThreadSubscribeResult,
        serialization: ThreadSharedRead,
    },
    ThreadUnsubscribe => "thread/unsubscribe" {
        params: ThreadUnsubscribeParams,
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
    TurnStart => "turn/start" {
        params: TurnStartParams,
        response: TurnStartResult,
        serialization: ThreadExclusive,
    },
    ShellTurnStart => "turn/shell/start" {
        params: ShellTurnStartParams,
        response: TurnStartResult,
        serialization: ThreadExclusive,
    },
    TurnInterrupt => "turn/interrupt" {
        params: TurnInterruptParams,
        response: TurnInterruptResult,
        serialization: ThreadExclusive,
    },
    TurnInteractionResolve => "turn/interaction/resolve" {
        params: TurnInteractionResolveParams,
        response: TurnInteractionResolveResult,
        serialization: ThreadExclusive,
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
    SessionUpdate => "session/update" {
        params: SessionUpdateEnvelope,
    },
    ThreadUpdate => "thread/update" {
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
    SchemaHash,
    ClientInfo,
    BrowserCapability,
    ClientCapabilities,
    ServerInfo,
    ModelRefDto,
    ApprovalReviewModelSelectionDto,
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
    SessionCommandParams,
    SessionModelSetParams,
    SessionThreadCreateParams,
    SessionThreadForkParams,
    SessionThreadRewindParams,
    SessionThreadArchiveParams,
    SessionResult,
    SessionListResult,
    SessionSubscribeResult,
    SessionThreadResult,
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
    ThreadEvent,
    PlanStepStatus,
    PlanStep,
    PlanUpdate,
    StreamCursor,
    ItemDelta,
    ThreadUpdate,
    ThreadUpdateEnvelope,
    ThreadReadParams,
    ThreadSubscribeParams,
    ThreadUnsubscribeParams,
    ThreadReadResult,
    ThreadSubscribeResult,
    InputItem,
    ShellTurnStartParams,
    TurnStartParams,
    TurnStartResult,
    TurnInterruptParams,
    TurnInterruptResult,
    TurnInteractionResolveParams,
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
    FsReadFileParams,
    FsReadFileResult,
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
