use crate::protocol::attachments::AttachmentImportRemoteParams;
use crate::protocol::attachments::AttachmentMaterializeResult;
use crate::protocol::attachments::AttachmentUploadCancelParams;
use crate::protocol::attachments::AttachmentUploadFinishParams;
use crate::protocol::attachments::AttachmentUploadStartParams;
use crate::protocol::attachments::AttachmentUploadStartResult;
use crate::protocol::attachments::AttachmentUploadWriteParams;
use crate::protocol::attachments::AttachmentUploadWriteResult;
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
use crate::protocol::config::ExecPolicyActionKindDto;
use crate::protocol::config::ExecPolicyEffectDto;
use crate::protocol::config::ExecPolicyHostMatcherDto;
use crate::protocol::config::ExecPolicyRuleDto;
use crate::protocol::config::ExecPolicyRuleRemoveParams;
use crate::protocol::config::ExecPolicyRuleUpsertParams;
use crate::protocol::config::ExecPolicyScopeMatcherDto;
use crate::protocol::config::ExecPolicySelectorDto;
use crate::protocol::config::ExecPolicyTokenDto;
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
use crate::protocol::connectors::ConnectorAccountDto;
use crate::protocol::connectors::ConnectorApiTokenConnectParams;
use crate::protocol::connectors::ConnectorAvailableActionDto;
use crate::protocol::connectors::ConnectorCommandDispositionDto;
use crate::protocol::connectors::ConnectorCommandResultDto;
use crate::protocol::connectors::ConnectorConnectionStateDto;
use crate::protocol::connectors::ConnectorCredentialCleanupDto;
use crate::protocol::connectors::ConnectorCredentialCleanupParams;
use crate::protocol::connectors::ConnectorDeviceOAuthPollParams;
use crate::protocol::connectors::ConnectorDeviceOAuthPollResult;
use crate::protocol::connectors::ConnectorDeviceOAuthStartParams;
use crate::protocol::connectors::ConnectorDeviceOAuthStartResult;
use crate::protocol::connectors::ConnectorDisconnectParams;
use crate::protocol::connectors::ConnectorDisconnectResultDto;
use crate::protocol::connectors::ConnectorDto;
use crate::protocol::connectors::ConnectorListResult;
use crate::protocol::connectors::ConnectorOAuthCancelParams;
use crate::protocol::connectors::ConnectorOAuthCompleteParams;
use crate::protocol::connectors::ConnectorOAuthMethodDto;
use crate::protocol::connectors::ConnectorOAuthRefreshParams;
use crate::protocol::connectors::ConnectorOAuthStartParams;
use crate::protocol::connectors::ConnectorOAuthStartResult;
use crate::protocol::connectors::ConnectorSecretDto;
use crate::protocol::connectors::ConnectorsChanged;
use crate::protocol::debug::DebugAdapterCloseParams;
use crate::protocol::debug::DebugAdapterMessageDto;
use crate::protocol::debug::DebugAdapterReadParams;
use crate::protocol::debug::DebugAdapterReadResult;
use crate::protocol::debug::DebugAdapterSendParams;
use crate::protocol::debug::DebugAdapterStartParams;
use crate::protocol::debug::DebugAdapterStartResult;
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
use crate::protocol::extension_host::ExtensionHostCancellationReasonDto;
use crate::protocol::extension_host::ExtensionHostChanged;
use crate::protocol::extension_host::ExtensionHostExtensionDto;
use crate::protocol::extension_host::ExtensionHostFailureCodeDto;
use crate::protocol::extension_host::ExtensionHostFailureDto;
use crate::protocol::extension_host::ExtensionHostInvokeCancelDispositionDto;
use crate::protocol::extension_host::ExtensionHostInvokeCancelParams;
use crate::protocol::extension_host::ExtensionHostInvokeCancelResult;
use crate::protocol::extension_host::ExtensionHostInvokeReadParams;
use crate::protocol::extension_host::ExtensionHostInvokeReadResult;
use crate::protocol::extension_host::ExtensionHostInvokeStartParams;
use crate::protocol::extension_host::ExtensionHostInvokeStartResult;
use crate::protocol::extension_host::ExtensionHostLanguageProviderOperationDto;
use crate::protocol::extension_host::ExtensionHostLifecycleDto;
use crate::protocol::extension_host::ExtensionHostReconcileModeDto;
use crate::protocol::extension_host::ExtensionHostReconcileParams;
use crate::protocol::extension_host::ExtensionHostRegistrationDescriptorDto;
use crate::protocol::extension_host::ExtensionHostRegistrationKindDto;
use crate::protocol::extension_host::ExtensionHostSnapshotDto;
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
    FsChanged, FsCreateFileParams, FsDeleteMode, FsDeleteParams, FsExistingTargetBehavior,
    FsFileType, FsGetMetadataParams, FsGetMetadataResult, FsMissingTargetBehavior,
    FsReadBinaryFileParams, FsReadBinaryFileResult, FsReadDirectoryEntry, FsReadDirectoryParams,
    FsReadDirectoryResult, FsReadFileParams, FsReadFileResult, FsRenameParams, FsWriteFileParams,
    FsWriteFileResult,
};
use crate::protocol::git::{
    GitBranchDto, GitBranchListResult, GitBranchSwitchParams, GitChangeStatusDto, GitCommitParams,
    GitCommitResult, GitCommitSummaryDto, GitDiffStatisticsDto, GitHeadDto, GitHistoryResult,
    GitOperationResult, GitPathsParams, GitRepositoryChangeDto, GitStatusChanged, GitStatusResult,
    GitSubmoduleStateDto, GitTextDiffDto, GitTextDiffResult, GitUpstreamDto,
};
use crate::protocol::initialize::{InitializeParams, InitializeResult, ServerCapabilities};
use crate::protocol::language::{
    LanguageCloseParams, LanguageCodeActionDiagnosticDto, LanguageCodeActionDto,
    LanguageCodeActionsParams, LanguageCodeActionsResult, LanguageCodeLensDto,
    LanguageCodeLensesResult, LanguageColorDto, LanguageColorPresentationDto,
    LanguageColorPresentationsParams, LanguageColorPresentationsResult, LanguageCommandDto,
    LanguageCompletionDetailsResult, LanguageCompletionInsertTextFormatDto,
    LanguageCompletionItemDto, LanguageCompletionItemKindDto, LanguageCompletionTriggerKindDto,
    LanguageCompletionsParams, LanguageCompletionsResult, LanguageDiagnosticReportKindDto,
    LanguageDiagnosticSeverityDto, LanguageDiagnosticsNotification, LanguageDocumentColorDto,
    LanguageDocumentColorsResult, LanguageDocumentDiagnosticsParams,
    LanguageDocumentDiagnosticsResult, LanguageDocumentDto, LanguageDocumentFeaturesParams,
    LanguageDocumentFormattingParams, LanguageDocumentLinkDto, LanguageDocumentLinksResult,
    LanguageDocumentSymbolDto, LanguageDocumentSymbolsResult, LanguageExecuteCommandParams,
    LanguageFoldingRangeDto, LanguageFoldingRangeKindDto, LanguageFoldingRangesResult,
    LanguageFormattingOptionsDto, LanguageFormattingResult, LanguageHierarchyEntryDto,
    LanguageHierarchyItemDto, LanguageHierarchyKindDto, LanguageHierarchyParams,
    LanguageHierarchyResultDto, LanguageHoverParams, LanguageHoverResult, LanguageInlayHintDto,
    LanguageInlayHintKindDto, LanguageInlayHintsParams, LanguageInlayHintsResult,
    LanguageLinkedEditingRangesParams, LanguageLinkedEditingRangesResult, LanguageLocationDto,
    LanguageLocationKindDto, LanguageLocationsParams, LanguageLocationsResult,
    LanguageParameterInformationDto, LanguagePositionDto, LanguagePrepareRenameParams,
    LanguagePrepareRenameResult, LanguageRangeDto, LanguageRangeFormattingParams,
    LanguageRenameParams, LanguageRenamePreparationDto, LanguageResolveCodeActionParams,
    LanguageResolveCodeLensParams, LanguageResolveCompletionParams,
    LanguageResolveDocumentLinkParams, LanguageSemanticTokenDto, LanguageSemanticTokensParams,
    LanguageSemanticTokensResult, LanguageServerMessageNotification,
    LanguageServerMessageSeverityDto, LanguageServerProgressNotification,
    LanguageSignatureHelpParams, LanguageSignatureHelpResult, LanguageSignatureHelpTriggerKindDto,
    LanguageSignatureInformationDto, LanguageSynchronizeParams, LanguageTextDocumentEditDto,
    LanguageTextEditDto, LanguageWorkspaceDiagnosticSnapshotDto,
    LanguageWorkspaceDiagnosticsParams, LanguageWorkspaceDiagnosticsResult,
    LanguageWorkspaceEditDto, LanguageWorkspaceEditEntryDto, LanguageWorkspaceSymbolDto,
    LanguageWorkspaceSymbolsParams, LanguageWorkspaceSymbolsResult,
};
use crate::protocol::model::{ModelCatalogEntry, ModelListResult};
use crate::protocol::notification::{SessionUpdateEnvelope, ThreadUpdateEnvelope};
use crate::protocol::plugins::PluginCommandDispositionDto;
use crate::protocol::plugins::PluginCommandResultDto;
use crate::protocol::plugins::PluginListResult;
use crate::protocol::plugins::PluginMarketplaceCommandParams;
use crate::protocol::plugins::PluginMarketplaceListResult;
use crate::protocol::plugins::PluginMarketplaceModeDto;
use crate::protocol::plugins::PluginMarketplacePackageDto;
use crate::protocol::plugins::PluginPackageCommandParams;
use crate::protocol::plugins::PluginPackageDto;
use crate::protocol::plugins::PluginsChanged;
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
    SkillDto, SkillEnablementDto, SkillListParams, SkillListResult, SkillResourceKindDto,
    SkillResourceOpenParams, SkillResourceOpenResult, SkillSetEnablementParams, SkillSourceKindDto,
    SkillsChanged,
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
use zeta_protocol::ApprovalMode;
use zeta_protocol::ContentPart;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::ImageDetail;
use zeta_protocol::ImageMediaType;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallCaller;
use zeta_protocol::ToolSourceProvenance;
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
    ConnectorList => "connector/list" {
        params: EmptyParams,
        response: ConnectorListResult,
        serialization: GlobalSharedRead,
    },
    ConnectorApiTokenConnect => "connector/connect/apiToken" {
        params: ConnectorApiTokenConnectParams,
        response: ConnectorCommandResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorOAuthStart => "connector/connect/oauth/start" {
        params: ConnectorOAuthStartParams,
        response: ConnectorOAuthStartResult,
        serialization: GlobalExclusive,
    },
    ConnectorOAuthComplete => "connector/connect/oauth/complete" {
        params: ConnectorOAuthCompleteParams,
        response: ConnectorCommandResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorOAuthCancel => "connector/connect/oauth/cancel" {
        params: ConnectorOAuthCancelParams,
        response: ConnectorCommandResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorDeviceOAuthStart => "connector/connect/oauth/device/start" {
        params: ConnectorDeviceOAuthStartParams,
        response: ConnectorDeviceOAuthStartResult,
        serialization: GlobalExclusive,
    },
    ConnectorDeviceOAuthPoll => "connector/connect/oauth/device/poll" {
        params: ConnectorDeviceOAuthPollParams,
        response: ConnectorDeviceOAuthPollResult,
        serialization: GlobalExclusive,
    },
    ConnectorDeviceOAuthCancel => "connector/connect/oauth/device/cancel" {
        params: ConnectorOAuthCancelParams,
        response: ConnectorCommandResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorOAuthRefresh => "connector/oauth/refresh" {
        params: ConnectorOAuthRefreshParams,
        response: (),
        serialization: GlobalExclusive,
    },
    ConnectorOAuthRevoke => "connector/oauth/revoke" {
        params: ConnectorDisconnectParams,
        response: ConnectorDisconnectResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorDisconnect => "connector/disconnect" {
        params: ConnectorDisconnectParams,
        response: ConnectorDisconnectResultDto,
        serialization: GlobalExclusive,
    },
    ConnectorCredentialCleanupRetry => "connector/credential/cleanup" {
        params: ConnectorCredentialCleanupParams,
        response: ConnectorCredentialCleanupDto,
        serialization: GlobalExclusive,
    },
    PluginList => "plugin/list" {
        params: EmptyParams,
        response: PluginListResult,
        serialization: GlobalSharedRead,
    },
    PluginMarketplaceList => "plugin/marketplace/list" {
        params: EmptyParams,
        response: PluginMarketplaceListResult,
        serialization: GlobalSharedRead,
    },
    PluginInstall => "plugin/install" {
        params: PluginMarketplaceCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginUpdate => "plugin/update" {
        params: PluginMarketplaceCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginRollback => "plugin/rollback" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginEnable => "plugin/enable" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginDisable => "plugin/disable" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginGrant => "plugin/grant" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginRevokeGrant => "plugin/revokeGrant" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
    },
    PluginUninstall => "plugin/uninstall" {
        params: PluginPackageCommandParams,
        response: PluginCommandResultDto,
        serialization: GlobalExclusive,
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
    ExecPolicyRuleUpsert => "execPolicy/rule/upsert" {
        params: ExecPolicyRuleUpsertParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    ExecPolicyRuleRemove => "execPolicy/rule/remove" {
        params: ExecPolicyRuleRemoveParams,
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
    SkillResourceOpen => "skill/resource/open" {
        params: SkillResourceOpenParams,
        response: SkillResourceOpenResult,
        serialization: ResourceExclusive,
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
    ExtensionHostList => "extensionHost/list" {
        params: EmptyParams,
        response: ExtensionHostSnapshotDto,
        serialization: GlobalSharedRead,
    },
    ExtensionHostReconcile => "extensionHost/reconcile" {
        params: ExtensionHostReconcileParams,
        response: ExtensionHostSnapshotDto,
        serialization: GlobalExclusive,
    },
    ExtensionHostInvokeStart => "extensionHost/invoke/start" {
        params: ExtensionHostInvokeStartParams,
        response: ExtensionHostInvokeStartResult,
        serialization: None,
    },
    ExtensionHostInvokeRead => "extensionHost/invoke/read" {
        params: ExtensionHostInvokeReadParams,
        response: ExtensionHostInvokeReadResult,
        serialization: None,
    },
    ExtensionHostInvokeCancel => "extensionHost/invoke/cancel" {
        params: ExtensionHostInvokeCancelParams,
        response: ExtensionHostInvokeCancelResult,
        serialization: None,
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
    AttachmentUploadStart => "attachment/upload/start" {
        params: AttachmentUploadStartParams,
        response: AttachmentUploadStartResult,
        serialization: ResourceExclusive,
    },
    AttachmentUploadWrite => "attachment/upload/write" {
        params: AttachmentUploadWriteParams,
        response: AttachmentUploadWriteResult,
        serialization: ResourceExclusive,
    },
    AttachmentUploadFinish => "attachment/upload/finish" {
        params: AttachmentUploadFinishParams,
        response: AttachmentMaterializeResult,
        serialization: ResourceExclusive,
    },
    AttachmentUploadCancel => "attachment/upload/cancel" {
        params: AttachmentUploadCancelParams,
        response: (),
        serialization: ResourceExclusive,
    },
    AttachmentImportRemote => "attachment/importRemote" {
        params: AttachmentImportRemoteParams,
        response: AttachmentMaterializeResult,
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
    LanguageSynchronize => "language/synchronize" {
        params: LanguageSynchronizeParams,
        response: (),
        serialization: GlobalSharedRead,
    },
    LanguageClose => "language/close" {
        params: LanguageCloseParams,
        response: (),
        serialization: GlobalSharedRead,
    },
    LanguageHover => "language/hover" {
        params: LanguageHoverParams,
        response: LanguageHoverResult,
        serialization: GlobalSharedRead,
    },
    LanguageCompletions => "language/completions" {
        params: LanguageCompletionsParams,
        response: LanguageCompletionsResult,
        serialization: GlobalSharedRead,
    },
    LanguageResolveCompletion => "language/resolveCompletion" {
        params: LanguageResolveCompletionParams,
        response: LanguageCompletionDetailsResult,
        serialization: GlobalSharedRead,
    },
    LanguageExecuteCommand => "language/executeCommand" {
        params: LanguageExecuteCommandParams,
        response: (),
        serialization: GlobalSharedRead,
    },
    LanguageDocumentDiagnostics => "language/documentDiagnostics" {
        params: LanguageDocumentDiagnosticsParams,
        response: LanguageDocumentDiagnosticsResult,
        serialization: GlobalSharedRead,
    },
    LanguageWorkspaceDiagnostics => "language/workspaceDiagnostics" {
        params: LanguageWorkspaceDiagnosticsParams,
        response: LanguageWorkspaceDiagnosticsResult,
        serialization: GlobalSharedRead,
    },
    LanguageLocations => "language/locations" {
        params: LanguageLocationsParams,
        response: LanguageLocationsResult,
        serialization: GlobalSharedRead,
    },
    LanguageHierarchy => "language/hierarchy" {
        params: LanguageHierarchyParams,
        response: LanguageHierarchyResultDto,
        serialization: GlobalSharedRead,
    },
    LanguageWorkspaceSymbols => "language/workspaceSymbols" {
        params: LanguageWorkspaceSymbolsParams,
        response: LanguageWorkspaceSymbolsResult,
        serialization: GlobalSharedRead,
    },
    LanguagePrepareRename => "language/prepareRename" {
        params: LanguagePrepareRenameParams,
        response: LanguagePrepareRenameResult,
        serialization: GlobalSharedRead,
    },
    LanguageRename => "language/rename" {
        params: LanguageRenameParams,
        response: LanguageWorkspaceEditDto,
        serialization: GlobalSharedRead,
    },
    LanguageCodeActions => "language/codeActions" {
        params: LanguageCodeActionsParams,
        response: LanguageCodeActionsResult,
        serialization: GlobalSharedRead,
    },
    LanguageResolveCodeAction => "language/resolveCodeAction" {
        params: LanguageResolveCodeActionParams,
        response: LanguageCodeActionDto,
        serialization: GlobalSharedRead,
    },
    LanguageDocumentFormatting => "language/formatDocument" {
        params: LanguageDocumentFormattingParams,
        response: LanguageFormattingResult,
        serialization: GlobalSharedRead,
    },
    LanguageRangeFormatting => "language/formatRange" {
        params: LanguageRangeFormattingParams,
        response: LanguageFormattingResult,
        serialization: GlobalSharedRead,
    },
    LanguageSignatureHelp => "language/signatureHelp" {
        params: LanguageSignatureHelpParams,
        response: LanguageSignatureHelpResult,
        serialization: GlobalSharedRead,
    },
    LanguageInlayHints => "language/inlayHints" {
        params: LanguageInlayHintsParams,
        response: LanguageInlayHintsResult,
        serialization: GlobalSharedRead,
    },
    LanguageLinkedEditingRanges => "language/linkedEditingRanges" {
        params: LanguageLinkedEditingRangesParams,
        response: LanguageLinkedEditingRangesResult,
        serialization: GlobalSharedRead,
    },
    LanguageSemanticTokens => "language/semanticTokens" {
        params: LanguageSemanticTokensParams,
        response: LanguageSemanticTokensResult,
        serialization: GlobalSharedRead,
    },
    LanguageDocumentSymbols => "language/documentSymbols" {
        params: LanguageDocumentFeaturesParams,
        response: LanguageDocumentSymbolsResult,
        serialization: GlobalSharedRead,
    },
    LanguageCodeLenses => "language/codeLenses" {
        params: LanguageDocumentFeaturesParams,
        response: LanguageCodeLensesResult,
        serialization: GlobalSharedRead,
    },
    LanguageResolveCodeLens => "language/resolveCodeLens" {
        params: LanguageResolveCodeLensParams,
        response: LanguageCodeLensesResult,
        serialization: GlobalSharedRead,
    },
    LanguageDocumentLinks => "language/documentLinks" {
        params: LanguageDocumentFeaturesParams,
        response: LanguageDocumentLinksResult,
        serialization: GlobalSharedRead,
    },
    LanguageResolveDocumentLink => "language/resolveDocumentLink" {
        params: LanguageResolveDocumentLinkParams,
        response: LanguageDocumentLinksResult,
        serialization: GlobalSharedRead,
    },
    LanguageDocumentColors => "language/documentColors" {
        params: LanguageDocumentFeaturesParams,
        response: LanguageDocumentColorsResult,
        serialization: GlobalSharedRead,
    },
    LanguageColorPresentations => "language/colorPresentations" {
        params: LanguageColorPresentationsParams,
        response: LanguageColorPresentationsResult,
        serialization: GlobalSharedRead,
    },
    LanguageFoldingRanges => "language/foldingRanges" {
        params: LanguageDocumentFeaturesParams,
        response: LanguageFoldingRangesResult,
        serialization: GlobalSharedRead,
    },
    FsWriteFile => "fs/writeFile" {
        params: FsWriteFileParams,
        response: FsWriteFileResult,
        serialization: GlobalExclusive,
    },
    FsCreateFile => "fs/createFile" {
        params: FsCreateFileParams,
        response: FsGetMetadataResult,
        serialization: GlobalExclusive,
    },
    FsRename => "fs/rename" {
        params: FsRenameParams,
        response: (),
        serialization: GlobalExclusive,
    },
    FsDelete => "fs/delete" {
        params: FsDeleteParams,
        response: (),
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
    DebugAdapterStart => "debug/adapter/start" {
        params: DebugAdapterStartParams,
        response: DebugAdapterStartResult,
        serialization: None,
    },
    DebugAdapterSend => "debug/adapter/send" {
        params: DebugAdapterSendParams,
        response: (),
        serialization: None,
    },
    DebugAdapterRead => "debug/adapter/read" {
        params: DebugAdapterReadParams,
        response: DebugAdapterReadResult,
        serialization: None,
    },
    DebugAdapterClose => "debug/adapter/close" {
        params: DebugAdapterCloseParams,
        response: (),
        serialization: None,
    },
}

macro_rules! server_notifications {
    (
        $(
            $variant:ident => $method:literal {
                params: $params:ty,
                $(storage: $storage:ident,)?
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

        /// A typed App Server notification decoded from the external wire contract.
        ///
        /// Consumers should project only the capabilities they own and retain a fallback arm.
        /// Adding a protocol notification is intentionally exhaustive only inside this crate.
        #[non_exhaustive]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum ServerNotification {
            $(
                $variant(notification_storage_type!($params $(, $storage)?)),
            )+
            Unknown {
                method: String,
                params: serde_json::Value,
            },
        }

        /// Decodes one registered notification payload while preserving unknown methods.
        pub fn decode_server_notification(
            method: String,
            params: serde_json::Value,
        ) -> Result<ServerNotification, serde_json::Error> {
            match server_notification_method(&method) {
                $(
                    Some(ServerNotificationMethod::$variant) => {
                        serde_json::from_value::<$params>(params).map(|payload| {
                            ServerNotification::$variant(notification_storage!(
                                payload $(, $storage)?
                            ))
                        })
                    }
                )+
                None => Ok(ServerNotification::Unknown { method, params }),
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

macro_rules! notification_storage_type {
    ($params:ty) => {
        $params
    };
    ($params:ty, boxed) => {
        Box<$params>
    };
}

macro_rules! notification_storage {
    ($payload:expr) => {
        $payload
    };
    ($payload:expr, boxed) => {
        Box::new($payload)
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
        storage: boxed,
    },
    ConfigChanged => "config/changed" {
        params: ConfigChanged,
    },
    ConnectorsChanged => "connector/changed" {
        params: ConnectorsChanged,
    },
    PluginsChanged => "plugin/changed" {
        params: PluginsChanged,
    },
    SkillsChanged => "skills/changed" {
        params: SkillsChanged,
    },
    ExtensionHostChanged => "extensionHost/changed" {
        params: ExtensionHostChanged,
    },
    GitStatusChanged => "git/statusChanged" {
        params: GitStatusChanged,
    },
    FsChanged => "fs/changed" {
        params: FsChanged,
    },
    LanguageDiagnostics => "language/diagnostics" {
        params: LanguageDiagnosticsNotification,
    },
    LanguageServerMessage => "language/serverMessage" {
        params: LanguageServerMessageNotification,
    },
    LanguageServerProgress => "language/serverProgress" {
        params: LanguageServerProgressNotification,
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
    ConnectorAccountDto,
    ConnectorAvailableActionDto,
    ConnectorOAuthMethodDto,
    ConnectorConnectionStateDto,
    ConnectorDto,
    ConnectorListResult,
    ConnectorSecretDto,
    ConnectorApiTokenConnectParams,
    ConnectorOAuthStartParams,
    ConnectorOAuthStartResult,
    ConnectorOAuthCompleteParams,
    ConnectorOAuthCancelParams,
    ConnectorDeviceOAuthStartParams,
    ConnectorDeviceOAuthStartResult,
    ConnectorDeviceOAuthPollParams,
    ConnectorDeviceOAuthPollResult,
    ConnectorOAuthRefreshParams,
    ConnectorDisconnectParams,
    ConnectorCommandDispositionDto,
    ConnectorCommandResultDto,
    ConnectorCredentialCleanupDto,
    ConnectorCredentialCleanupParams,
    ConnectorDisconnectResultDto,
    ConnectorsChanged,
    PluginPackageDto,
    PluginListResult,
    PluginMarketplaceCommandParams,
    PluginMarketplaceListResult,
    PluginMarketplaceModeDto,
    PluginMarketplacePackageDto,
    PluginPackageCommandParams,
    PluginCommandDispositionDto,
    PluginCommandResultDto,
    PluginsChanged,
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
    ExecPolicyActionKindDto,
    ExecPolicyTokenDto,
    ExecPolicyHostMatcherDto,
    ExecPolicyScopeMatcherDto,
    ExecPolicySelectorDto,
    ExecPolicyEffectDto,
    ExecPolicyRuleDto,
    ExecPolicyRuleUpsertParams,
    ExecPolicyRuleRemoveParams,
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
    SkillResourceKindDto,
    SkillResourceOpenParams,
    SkillResourceOpenResult,
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
    ExtensionHostReconcileModeDto,
    ExtensionHostReconcileParams,
    ExtensionHostSnapshotDto,
    ExtensionHostExtensionDto,
    ExtensionHostLifecycleDto,
    ExtensionHostFailureCodeDto,
    ExtensionHostFailureDto,
    ExtensionHostRegistrationDescriptorDto,
    ExtensionHostRegistrationKindDto,
    ExtensionHostLanguageProviderOperationDto,
    ExtensionHostInvokeStartParams,
    ExtensionHostInvokeStartResult,
    ExtensionHostInvokeReadParams,
    ExtensionHostInvokeReadResult,
    ExtensionHostInvokeCancelParams,
    ExtensionHostInvokeCancelResult,
    ExtensionHostInvokeCancelDispositionDto,
    ExtensionHostCancellationReasonDto,
    ExtensionHostChanged,
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
    ApprovalMode,
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
    ToolCallBinding,
    ToolSourceProvenance,
    ToolCallCaller,
    ContentPart,
    ImageAttachmentRef,
    ImageMediaType,
    ImageDetail,
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
    AttachmentUploadStartParams,
    AttachmentUploadStartResult,
    AttachmentUploadWriteParams,
    AttachmentUploadWriteResult,
    AttachmentUploadFinishParams,
    AttachmentUploadCancelParams,
    AttachmentImportRemoteParams,
    AttachmentMaterializeResult,
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
    LanguageLocationKindDto,
    LanguagePositionDto,
    LanguageRangeDto,
    LanguageDocumentDto,
    LanguageSynchronizeParams,
    LanguageCloseParams,
    LanguageHoverParams,
    LanguageHoverResult,
    LanguageCompletionTriggerKindDto,
    LanguageCompletionsParams,
    LanguageCompletionItemKindDto,
    LanguageCompletionInsertTextFormatDto,
    LanguageCompletionItemDto,
    LanguageResolveCompletionParams,
    LanguageCompletionDetailsResult,
    LanguageExecuteCommandParams,
    LanguageCompletionsResult,
    LanguageDocumentDiagnosticsParams,
    LanguageDiagnosticReportKindDto,
    LanguageDocumentDiagnosticsResult,
    LanguageWorkspaceDiagnosticsParams,
    LanguageWorkspaceDiagnosticSnapshotDto,
    LanguageWorkspaceDiagnosticsResult,
    LanguageFormattingOptionsDto,
    LanguageDocumentFormattingParams,
    LanguageRangeFormattingParams,
    LanguageFormattingResult,
    LanguageSignatureHelpTriggerKindDto,
    LanguageSignatureHelpParams,
    LanguageParameterInformationDto,
    LanguageSignatureInformationDto,
    LanguageSignatureHelpResult,
    LanguageInlayHintsParams,
    LanguageInlayHintKindDto,
    LanguageInlayHintDto,
    LanguageInlayHintsResult,
    LanguageLinkedEditingRangesParams,
    LanguageLinkedEditingRangesResult,
    LanguageSemanticTokensParams,
    LanguageSemanticTokenDto,
    LanguageSemanticTokensResult,
    LanguageDocumentFeaturesParams,
    LanguageDocumentSymbolDto,
    LanguageDocumentSymbolsResult,
    LanguageCommandDto,
    LanguageCodeLensDto,
    LanguageCodeLensesResult,
    LanguageResolveCodeLensParams,
    LanguageDocumentLinkDto,
    LanguageDocumentLinksResult,
    LanguageResolveDocumentLinkParams,
    LanguageColorDto,
    LanguageDocumentColorDto,
    LanguageDocumentColorsResult,
    LanguageColorPresentationsParams,
    LanguageColorPresentationDto,
    LanguageColorPresentationsResult,
    LanguageFoldingRangeKindDto,
    LanguageFoldingRangeDto,
    LanguageFoldingRangesResult,
    LanguageLocationsParams,
    LanguageLocationDto,
    LanguageLocationsResult,
    LanguageHierarchyKindDto,
    LanguageHierarchyItemDto,
    LanguageHierarchyParams,
    LanguageHierarchyEntryDto,
    LanguageHierarchyResultDto,
    LanguageWorkspaceSymbolsParams,
    LanguageWorkspaceSymbolDto,
    LanguageWorkspaceSymbolsResult,
    LanguagePrepareRenameParams,
    LanguageRenamePreparationDto,
    LanguagePrepareRenameResult,
    LanguageRenameParams,
    LanguageTextEditDto,
    LanguageTextDocumentEditDto,
    LanguageWorkspaceEditDto,
    LanguageWorkspaceEditEntryDto,
    LanguageDiagnosticSeverityDto,
    LanguageCodeActionDiagnosticDto,
    LanguageDiagnosticsNotification,
    LanguageServerMessageSeverityDto,
    LanguageServerMessageNotification,
    LanguageServerProgressNotification,
    LanguageCodeActionsParams,
    LanguageCodeActionDto,
    LanguageCodeActionsResult,
    LanguageResolveCodeActionParams,
    FsWriteFileParams,
    FsWriteFileResult,
    FsExistingTargetBehavior,
    FsMissingTargetBehavior,
    FsDeleteMode,
    FsCreateFileParams,
    FsRenameParams,
    FsDeleteParams,
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
    DebugAdapterStartParams,
    DebugAdapterStartResult,
    DebugAdapterSendParams,
    DebugAdapterReadParams,
    DebugAdapterMessageDto,
    DebugAdapterReadResult,
    DebugAdapterCloseParams,
    AppServerErrorName,
    AppServerError,
}
