use crate::protocol::account::AccountDto;
use crate::protocol::account::AccountLoginCancelParams;
use crate::protocol::account::AccountLoginCancelResult;
use crate::protocol::account::AccountLoginCancelStatusDto;
use crate::protocol::account::AccountLoginCompleted;
use crate::protocol::account::AccountLoginCompletionStatusDto;
use crate::protocol::account::AccountLoginFailureDto;
use crate::protocol::account::AccountLoginMethodDto;
use crate::protocol::account::AccountLoginStartParams;
use crate::protocol::account::AccountLoginStartResult;
use crate::protocol::account::AccountLogoutParams;
use crate::protocol::account::AccountLogoutResult;
use crate::protocol::account::AccountLogoutStatusDto;
use crate::protocol::account::AccountReadResult;
use crate::protocol::account::AccountStatusDto;
use crate::protocol::account::AccountUpdated;
use crate::protocol::attachments::AttachmentImportRemoteParams;
use crate::protocol::attachments::AttachmentMaterializeResult;
use crate::protocol::attachments::AttachmentUploadCancelParams;
use crate::protocol::attachments::AttachmentUploadFinishParams;
use crate::protocol::attachments::AttachmentUploadStartParams;
use crate::protocol::attachments::AttachmentUploadStartResult;
use crate::protocol::attachments::AttachmentUploadWriteParams;
use crate::protocol::attachments::AttachmentUploadWriteResult;
use crate::protocol::browser::BrowserBinaryPayload;
use crate::protocol::browser::BrowserCloseParams;
use crate::protocol::browser::BrowserCreateParams;
use crate::protocol::browser::BrowserCreateResult;
use crate::protocol::browser::BrowserElementTargetDto;
use crate::protocol::browser::BrowserObserveParams;
use crate::protocol::browser::BrowserObserveResult;
use crate::protocol::browser::BrowserPerformActionDto;
use crate::protocol::browser::BrowserPerformParams;
use crate::protocol::browser::BrowserPerformResult;
use crate::protocol::browser::BrowserTextInputTargetDto;
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
use crate::protocol::code_index::FastRegexDisableAndDeleteParams;
use crate::protocol::code_index::FastRegexDisableAndDeleteResult;
use crate::protocol::code_index::FastRegexIndexStatusResult;
use crate::protocol::code_index::LocalIndexClearOutcomeDto;
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
use crate::protocol::common::BrowserCapability;
use crate::protocol::common::ClientCapabilities;
use crate::protocol::common::ClientInfo;
use crate::protocol::common::CommandId;
use crate::protocol::common::EmptyParams;
use crate::protocol::common::ItemId;
use crate::protocol::common::RequestId;
use crate::protocol::common::SchemaHash;
use crate::protocol::common::ServerInfo;
use crate::protocol::common::SessionId;
use crate::protocol::common::StreamInstanceId;
use crate::protocol::common::ThreadId;
use crate::protocol::common::ToolCallId;
use crate::protocol::common::ToolName;
use crate::protocol::common::TurnId;
use crate::protocol::common::WorkspaceTrustHostCapability;
use crate::protocol::config::AgentGrepBackendDto;
use crate::protocol::config::ApprovalReviewModelSelectionDto;
use crate::protocol::config::ConfigChanged;
use crate::protocol::config::ConfigCommandDispositionDto;
use crate::protocol::config::ConfigCommandResult;
use crate::protocol::config::ConfigReadResult;
use crate::protocol::config::ConfigUpdateParams;
use crate::protocol::config::ExecPolicyActionKindDto;
use crate::protocol::config::ExecPolicyEffectDto;
use crate::protocol::config::ExecPolicyHostMatcherDto;
use crate::protocol::config::ExecPolicyRuleDto;
use crate::protocol::config::ExecPolicyRuleRemoveParams;
use crate::protocol::config::ExecPolicyRuleUpsertParams;
use crate::protocol::config::ExecPolicyScopeMatcherDto;
use crate::protocol::config::ExecPolicySelectorDto;
use crate::protocol::config::ExecPolicyTokenDto;
use crate::protocol::config::HookActionDto;
use crate::protocol::config::HookConfigDto;
use crate::protocol::config::HookEnablementDto;
use crate::protocol::config::HookEventDto;
use crate::protocol::config::HookMatcherDto;
use crate::protocol::config::HookRemoveParams;
use crate::protocol::config::HookSetEnablementParams;
use crate::protocol::config::HookUpsertParams;
use crate::protocol::config::LanguageServerConfigDto;
use crate::protocol::config::LanguageServerConfigureParams;
use crate::protocol::config::LanguageServerModeDto;
use crate::protocol::config::LanguageServerRemoveParams;
use crate::protocol::config::McpCredentialBindingDto;
use crate::protocol::config::McpServerConfigDto;
use crate::protocol::config::McpServerEnablementDto;
use crate::protocol::config::McpServerRemoveParams;
use crate::protocol::config::McpServerSetEnablementParams;
use crate::protocol::config::McpServerUpsertParams;
use crate::protocol::config::McpTransportDto;
use crate::protocol::config::ModelContextConfigDto;
use crate::protocol::config::ModelRefDto;
use crate::protocol::config::PluginRequestDto;
use crate::protocol::config::PluginRequestEnablementDto;
use crate::protocol::config::PluginRequestRemoveParams;
use crate::protocol::config::PluginRequestSetEnablementParams;
use crate::protocol::config::PluginRequestUpsertParams;
use crate::protocol::config::ProviderConfigDto;
use crate::protocol::config::ProviderConfigureParams;
use crate::protocol::config::ProviderRemoveParams;
use crate::protocol::config::SemanticCodeIndexAuthorizeParams;
use crate::protocol::config::SemanticCodeIndexAutomaticContextDto;
use crate::protocol::config::SemanticCodeIndexConfigDto;
use crate::protocol::config::SemanticCodeIndexConfigureParams;
use crate::protocol::config::SemanticCodeIndexModelsDto;
use crate::protocol::config::SemanticCodeIndexRevokeParams;
use crate::protocol::config::SemanticCodeIndexSelectionDto;
use crate::protocol::config::SkillSourceAddParams;
use crate::protocol::config::SkillSourceConfigDto;
use crate::protocol::config::SkillSourceEnablementDto;
use crate::protocol::config::SkillSourceRemoveParams;
use crate::protocol::config::SkillSourceSetEnablementParams;
use crate::protocol::config::ToolSearchConfigDto;
use crate::protocol::config::ToolSearchConfigureParams;
use crate::protocol::config::ToolSearchEmbeddingStatusDto;
use crate::protocol::config::ToolSearchModeDto;
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
use crate::protocol::document::TypstCompileParams;
use crate::protocol::document::TypstCompileResult;
use crate::protocol::document::TypstDiagnosticDto;
use crate::protocol::document::TypstDiagnosticSeverityDto;
use crate::protocol::document::TypstSourceRangeDto;
use crate::protocol::error::AppServerError;
use crate::protocol::error::AppServerErrorName;
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
use crate::protocol::extension_host::ExtensionHostOutputChannelKindDto;
use crate::protocol::extension_host::ExtensionHostOutputEventDto;
use crate::protocol::extension_host::ExtensionHostOutputOperationDto;
use crate::protocol::extension_host::ExtensionHostOutputSeverityDto;
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
use crate::protocol::fs::FsChanged;
use crate::protocol::fs::FsCreateFileParams;
use crate::protocol::fs::FsDeleteMode;
use crate::protocol::fs::FsDeleteParams;
use crate::protocol::fs::FsExistingTargetBehavior;
use crate::protocol::fs::FsFileType;
use crate::protocol::fs::FsGetMetadataParams;
use crate::protocol::fs::FsGetMetadataResult;
use crate::protocol::fs::FsMissingTargetBehavior;
use crate::protocol::fs::FsReadBinaryFileParams;
use crate::protocol::fs::FsReadBinaryFileResult;
use crate::protocol::fs::FsReadDirectoryEntry;
use crate::protocol::fs::FsReadDirectoryParams;
use crate::protocol::fs::FsReadDirectoryResult;
use crate::protocol::fs::FsReadFileParams;
use crate::protocol::fs::FsReadFileResult;
use crate::protocol::fs::FsRenameParams;
use crate::protocol::fs::FsWriteFileParams;
use crate::protocol::fs::FsWriteFileResult;
use crate::protocol::git::GitBranchDto;
use crate::protocol::git::GitBranchListResult;
use crate::protocol::git::GitBranchSwitchParams;
use crate::protocol::git::GitChangeFileComparisonDto;
use crate::protocol::git::GitChangeFileParams;
use crate::protocol::git::GitChangeFileResult;
use crate::protocol::git::GitChangeStatusDto;
use crate::protocol::git::GitCommitChangeDto;
use crate::protocol::git::GitCommitChangesParams;
use crate::protocol::git::GitCommitChangesResult;
use crate::protocol::git::GitCommitFileContentDto;
use crate::protocol::git::GitCommitFileParams;
use crate::protocol::git::GitCommitFileResult;
use crate::protocol::git::GitCommitParams;
use crate::protocol::git::GitCommitResult;
use crate::protocol::git::GitCommitSummaryDto;
use crate::protocol::git::GitDiffStatisticsDto;
use crate::protocol::git::GitGraphParams;
use crate::protocol::git::GitGraphResult;
use crate::protocol::git::GitHeadDto;
use crate::protocol::git::GitHistoryResult;
use crate::protocol::git::GitOperationResult;
use crate::protocol::git::GitPathsParams;
use crate::protocol::git::GitReferenceDto;
use crate::protocol::git::GitReferenceKindDto;
use crate::protocol::git::GitRemoteDto;
use crate::protocol::git::GitRemoteProviderDto;
use crate::protocol::git::GitRepositoriesResult;
use crate::protocol::git::GitRepositoryChangeDto;
use crate::protocol::git::GitRepositoryDto;
use crate::protocol::git::GitRepositoryIdentityDto;
use crate::protocol::git::GitRepositoryParams;
use crate::protocol::git::GitStatusChanged;
use crate::protocol::git::GitStatusResult;
use crate::protocol::git::GitSubmoduleStateDto;
use crate::protocol::git::GitTextDiffDto;
use crate::protocol::git::GitTextDiffResult;
use crate::protocol::git::GitUpstreamDto;
use crate::protocol::goal::ThreadGoalClearParams;
use crate::protocol::goal::ThreadGoalClearResponse;
use crate::protocol::goal::ThreadGoalClearedNotification;
use crate::protocol::goal::ThreadGoalGetParams;
use crate::protocol::goal::ThreadGoalGetResponse;
use crate::protocol::goal::ThreadGoalSetParams;
use crate::protocol::goal::ThreadGoalSetResponse;
use crate::protocol::goal::ThreadGoalUpdatedNotification;
use crate::protocol::initialize::CapabilityContract;
use crate::protocol::initialize::InitializeParams;
use crate::protocol::initialize::InitializeResult;
use crate::protocol::initialize::ProtocolVersion;
use crate::protocol::initialize::ServerCapabilities;
use crate::protocol::language::LanguageCloseParams;
use crate::protocol::language::LanguageCodeActionDiagnosticDto;
use crate::protocol::language::LanguageCodeActionDto;
use crate::protocol::language::LanguageCodeActionsParams;
use crate::protocol::language::LanguageCodeActionsResult;
use crate::protocol::language::LanguageCodeLensDto;
use crate::protocol::language::LanguageCodeLensesResult;
use crate::protocol::language::LanguageColorDto;
use crate::protocol::language::LanguageColorPresentationDto;
use crate::protocol::language::LanguageColorPresentationsParams;
use crate::protocol::language::LanguageColorPresentationsResult;
use crate::protocol::language::LanguageCommandDto;
use crate::protocol::language::LanguageCompletionDetailsResult;
use crate::protocol::language::LanguageCompletionInsertTextFormatDto;
use crate::protocol::language::LanguageCompletionItemDto;
use crate::protocol::language::LanguageCompletionItemKindDto;
use crate::protocol::language::LanguageCompletionTriggerKindDto;
use crate::protocol::language::LanguageCompletionsParams;
use crate::protocol::language::LanguageCompletionsResult;
use crate::protocol::language::LanguageDiagnosticReportKindDto;
use crate::protocol::language::LanguageDiagnosticSeverityDto;
use crate::protocol::language::LanguageDiagnosticsNotification;
use crate::protocol::language::LanguageDocumentColorDto;
use crate::protocol::language::LanguageDocumentColorsResult;
use crate::protocol::language::LanguageDocumentDiagnosticsParams;
use crate::protocol::language::LanguageDocumentDiagnosticsResult;
use crate::protocol::language::LanguageDocumentDto;
use crate::protocol::language::LanguageDocumentFeaturesParams;
use crate::protocol::language::LanguageDocumentFormattingParams;
use crate::protocol::language::LanguageDocumentLinkDto;
use crate::protocol::language::LanguageDocumentLinksResult;
use crate::protocol::language::LanguageDocumentSymbolDto;
use crate::protocol::language::LanguageDocumentSymbolsResult;
use crate::protocol::language::LanguageExecuteCommandParams;
use crate::protocol::language::LanguageFoldingRangeDto;
use crate::protocol::language::LanguageFoldingRangeKindDto;
use crate::protocol::language::LanguageFoldingRangesResult;
use crate::protocol::language::LanguageFormattingOptionsDto;
use crate::protocol::language::LanguageFormattingResult;
use crate::protocol::language::LanguageHierarchyEntryDto;
use crate::protocol::language::LanguageHierarchyItemDto;
use crate::protocol::language::LanguageHierarchyKindDto;
use crate::protocol::language::LanguageHierarchyParams;
use crate::protocol::language::LanguageHierarchyResultDto;
use crate::protocol::language::LanguageHoverParams;
use crate::protocol::language::LanguageHoverResult;
use crate::protocol::language::LanguageInlayHintDto;
use crate::protocol::language::LanguageInlayHintKindDto;
use crate::protocol::language::LanguageInlayHintsParams;
use crate::protocol::language::LanguageInlayHintsResult;
use crate::protocol::language::LanguageLinkedEditingRangesParams;
use crate::protocol::language::LanguageLinkedEditingRangesResult;
use crate::protocol::language::LanguageLocationDto;
use crate::protocol::language::LanguageLocationKindDto;
use crate::protocol::language::LanguageLocationsParams;
use crate::protocol::language::LanguageLocationsResult;
use crate::protocol::language::LanguageParameterInformationDto;
use crate::protocol::language::LanguagePositionDto;
use crate::protocol::language::LanguagePrepareRenameParams;
use crate::protocol::language::LanguagePrepareRenameResult;
use crate::protocol::language::LanguageRangeDto;
use crate::protocol::language::LanguageRangeFormattingParams;
use crate::protocol::language::LanguageRenameParams;
use crate::protocol::language::LanguageRenamePreparationDto;
use crate::protocol::language::LanguageResolveCodeActionParams;
use crate::protocol::language::LanguageResolveCodeLensParams;
use crate::protocol::language::LanguageResolveCompletionParams;
use crate::protocol::language::LanguageResolveDocumentLinkParams;
use crate::protocol::language::LanguageSemanticTokenDto;
use crate::protocol::language::LanguageSemanticTokensParams;
use crate::protocol::language::LanguageSemanticTokensResult;
use crate::protocol::language::LanguageServerMessageNotification;
use crate::protocol::language::LanguageServerMessageSeverityDto;
use crate::protocol::language::LanguageServerMessageSourceDto;
use crate::protocol::language::LanguageServerProgressNotification;
use crate::protocol::language::LanguageServerStateDto;
use crate::protocol::language::LanguageServerStateNotification;
use crate::protocol::language::LanguageSignatureHelpParams;
use crate::protocol::language::LanguageSignatureHelpResult;
use crate::protocol::language::LanguageSignatureHelpTriggerKindDto;
use crate::protocol::language::LanguageSignatureInformationDto;
use crate::protocol::language::LanguageSynchronizeParams;
use crate::protocol::language::LanguageTextDocumentEditDto;
use crate::protocol::language::LanguageTextEditDto;
use crate::protocol::language::LanguageWorkspaceDiagnosticSnapshotDto;
use crate::protocol::language::LanguageWorkspaceDiagnosticsParams;
use crate::protocol::language::LanguageWorkspaceDiagnosticsResult;
use crate::protocol::language::LanguageWorkspaceEditDto;
use crate::protocol::language::LanguageWorkspaceEditEntryDto;
use crate::protocol::language::LanguageWorkspaceSymbolDto;
use crate::protocol::language::LanguageWorkspaceSymbolsParams;
use crate::protocol::language::LanguageWorkspaceSymbolsResult;
use crate::protocol::marketplace::MarketplaceAcquireCapabilityParams;
use crate::protocol::marketplace::MarketplaceAcquiredCapabilityDto;
use crate::protocol::marketplace::MarketplaceActivationSpecDto;
use crate::protocol::marketplace::MarketplaceArtifactHandleDto;
use crate::protocol::marketplace::MarketplaceAvailableCapabilityDto;
use crate::protocol::marketplace::MarketplaceCapabilityDescriptorDto;
use crate::protocol::marketplace::MarketplaceCapabilityKindDto;
use crate::protocol::marketplace::MarketplaceCapabilityLeaseDto;
use crate::protocol::marketplace::MarketplaceCapabilityRefDto;
use crate::protocol::marketplace::MarketplaceChanged;
use crate::protocol::marketplace::MarketplaceConnectorActivationSpecDto;
use crate::protocol::marketplace::MarketplaceDownloadParams;
use crate::protocol::marketplace::MarketplaceExecutableActivationSpecDto;
use crate::protocol::marketplace::MarketplaceExecutableRuntimeDto;
use crate::protocol::marketplace::MarketplaceGetParams;
use crate::protocol::marketplace::MarketplaceInstallParams;
use crate::protocol::marketplace::MarketplaceInstallationStateDto;
use crate::protocol::marketplace::MarketplaceInstalledPackageDto;
use crate::protocol::marketplace::MarketplaceLanguageActivationSpecDto;
use crate::protocol::marketplace::MarketplaceListInstalledResult;
use crate::protocol::marketplace::MarketplaceLocalizationActivationSpecDto;
use crate::protocol::marketplace::MarketplaceMcpActivationSpecDto;
use crate::protocol::marketplace::MarketplaceMcpTransportDto;
use crate::protocol::marketplace::MarketplaceOpenResourceParams;
use crate::protocol::marketplace::MarketplacePackageDetailsDto;
use crate::protocol::marketplace::MarketplacePackageRefDto;
use crate::protocol::marketplace::MarketplacePackageSourceDto;
use crate::protocol::marketplace::MarketplacePackageSummaryDto;
use crate::protocol::marketplace::MarketplaceReleaseCapabilityParams;
use crate::protocol::marketplace::MarketplaceResourceContentDto;
use crate::protocol::marketplace::MarketplaceResourceRefDto;
use crate::protocol::marketplace::MarketplaceSearchParams;
use crate::protocol::marketplace::MarketplaceSearchResult;
use crate::protocol::marketplace::MarketplaceSkillActivationSpecDto;
use crate::protocol::marketplace::MarketplaceThemeActivationSpecDto;
use crate::protocol::marketplace::MarketplaceUninstallModeDto;
use crate::protocol::marketplace::MarketplaceUninstallParams;
use crate::protocol::marketplace::MarketplaceUpdateParams;
use crate::protocol::marketplace::MarketplaceUpstreamReferenceDto;
use crate::protocol::marketplace::MarketplaceUpstreamRegistryDto;
use crate::protocol::mcp::McpOAuthCompleteParams;
use crate::protocol::mcp::McpOAuthMutationParams;
use crate::protocol::mcp::McpOAuthMutationResult;
use crate::protocol::mcp::McpOAuthStartParams;
use crate::protocol::mcp::McpOAuthStartResult;
use crate::protocol::mcp::McpSecretDto;
use crate::protocol::mcp::McpServerRuntimeIntentDto;
use crate::protocol::mcp::McpServerRuntimeIntentParams;
use crate::protocol::mcp::McpServerRuntimeIntentResult;
use crate::protocol::mcp::McpServerRuntimeStateDto;
use crate::protocol::mcp::McpServerStatusDto;
use crate::protocol::mcp::McpServerStatusResult;
use crate::protocol::model::ModelCatalogEntry;
use crate::protocol::model::ModelListResult;
use crate::protocol::notification::SessionUpdateEnvelope;
use crate::protocol::notification::ThreadTranscriptUpdateEnvelope;
use crate::protocol::notification::ThreadUpdateEnvelope;
use crate::protocol::plugins::PluginCommandDispositionDto;
use crate::protocol::plugins::PluginCommandResultDto;
use crate::protocol::plugins::PluginListResult;
use crate::protocol::plugins::PluginPackageCommandParams;
use crate::protocol::plugins::PluginPackageDto;
use crate::protocol::plugins::PluginsChanged;
use crate::protocol::provider::ProviderApiKeyDto;
use crate::protocol::provider::ProviderApiKeyPolicyDto;
use crate::protocol::provider::ProviderApiKeySetParams;
use crate::protocol::provider::ProviderApiKeySetResult;
use crate::protocol::provider::ProviderCatalogEntryDto;
use crate::protocol::provider::ProviderListResult;
use crate::protocol::resources::ResourceMetadataParams;
use crate::protocol::resources::ResourceMetadataResult;
use crate::protocol::resources::ResourceReadParams;
use crate::protocol::resources::ResourceReadResult;
use crate::protocol::resources::ResourceReleaseParams;
use crate::protocol::search::WorkspaceSearchCancelParams;
use crate::protocol::search::WorkspaceSearchCaseSensitivity;
use crate::protocol::search::WorkspaceSearchMatch;
use crate::protocol::search::WorkspaceSearchMatchRange;
use crate::protocol::search::WorkspaceSearchPatternKind;
use crate::protocol::search::WorkspaceSearchReadParams;
use crate::protocol::search::WorkspaceSearchReadResult;
use crate::protocol::search::WorkspaceSearchStartParams;
use crate::protocol::search::WorkspaceSearchStartResult;
use crate::protocol::session::SessionCreateParams;
use crate::protocol::session::SessionListResult;
use crate::protocol::session::SessionReadParams;
use crate::protocol::session::SessionRequest;
use crate::protocol::session::SessionRequestParams;
use crate::protocol::session::SessionRequestResult;
use crate::protocol::session::SessionResult;
use crate::protocol::session::SessionRewriteResult;
use crate::protocol::session::SessionSubscribeParams;
use crate::protocol::session::SessionSubscribeResult;
use crate::protocol::session::SessionThreadProjection;
use crate::protocol::session::SessionThreadReadParams;
use crate::protocol::session::SessionThreadReadResult;
use crate::protocol::session::SessionThreadResult;
use crate::protocol::session::SessionThreadSubscribeParams;
use crate::protocol::session::SessionThreadSubscribeResult;
use crate::protocol::session::SessionThreadUnsubscribeParams;
use crate::protocol::session::SessionUnsubscribeParams;
use crate::protocol::session::ThreadHistoryBoundary;
use crate::protocol::session::ThreadSnapshotHistory;
use crate::protocol::skills::SkillCatalogReloadDto;
use crate::protocol::skills::SkillCompatibilityDto;
use crate::protocol::skills::SkillDiagnosticCodeDto;
use crate::protocol::skills::SkillDiagnosticDto;
use crate::protocol::skills::SkillDto;
use crate::protocol::skills::SkillEnablementDto;
use crate::protocol::skills::SkillListParams;
use crate::protocol::skills::SkillListResult;
use crate::protocol::skills::SkillResourceKindDto;
use crate::protocol::skills::SkillResourceOpenParams;
use crate::protocol::skills::SkillResourceOpenResult;
use crate::protocol::skills::SkillSetEnablementParams;
use crate::protocol::skills::SkillSourceKindDto;
use crate::protocol::skills::SkillsChanged;
use crate::protocol::slash_commands::SlashCommandArgumentModeDto;
use crate::protocol::slash_commands::SlashCommandDefinition;
use crate::protocol::symbol_index::SymbolIndexSearchHitDto;
use crate::protocol::symbol_index::SymbolIndexSearchParams;
use crate::protocol::symbol_index::SymbolIndexSearchResult;
use crate::protocol::symbol_index::SymbolIndexStateDto;
use crate::protocol::symbol_index::SymbolIndexStatusResult;
use crate::protocol::symbol_index::SymbolKindDto;
use crate::protocol::symbol_index::WorkspaceDocumentOverlayCloseParams;
use crate::protocol::symbol_index::WorkspaceDocumentOverlayStatusResult;
use crate::protocol::symbol_index::WorkspaceDocumentOverlaySynchronizeParams;
use crate::protocol::syntax::SyntaxAnalyzeParams;
use crate::protocol::syntax::SyntaxAnalyzeResult;
use crate::protocol::syntax::SyntaxDiagnosticDto;
use crate::protocol::syntax::SyntaxDiagnosticKindDto;
use crate::protocol::syntax::SyntaxFoldingRangeDto;
use crate::protocol::syntax::SyntaxLanguageDto;
use crate::protocol::syntax::SyntaxPositionDto;
use crate::protocol::syntax::SyntaxRangeDto;
use crate::protocol::syntax::SyntaxSelectionRangeDto;
use crate::protocol::syntax::SyntaxSelectionRangesParams;
use crate::protocol::syntax::SyntaxSelectionRangesResult;
use crate::protocol::syntax::SyntaxSymbolDto;
use crate::protocol::syntax::SyntaxSymbolKindDto;
use crate::protocol::syntax::SyntaxTokenDto;
use crate::protocol::syntax::SyntaxTokenKindDto;
use crate::protocol::terminal::TerminalAttachParams;
use crate::protocol::terminal::TerminalAttachResult;
use crate::protocol::terminal::TerminalCloseParams;
use crate::protocol::terminal::TerminalCommandStatus;
use crate::protocol::terminal::TerminalCommandStatusEvent;
use crate::protocol::terminal::TerminalCreateInSessionDirectoryParams;
use crate::protocol::terminal::TerminalCreateParams;
use crate::protocol::terminal::TerminalCreateResult;
use crate::protocol::terminal::TerminalLifecycle;
use crate::protocol::terminal::TerminalOutputChunk;
use crate::protocol::terminal::TerminalProfile;
use crate::protocol::terminal::TerminalProfileListResult;
use crate::protocol::terminal::TerminalProfileSelection;
use crate::protocol::terminal::TerminalReadParams;
use crate::protocol::terminal::TerminalReadResult;
use crate::protocol::terminal::TerminalReconnectLease;
use crate::protocol::terminal::TerminalResizeParams;
use crate::protocol::terminal::TerminalWriteParams;
use crate::protocol::transcript::ThreadTranscriptChange;
use crate::protocol::transcript::ThreadTranscriptEntry;
use crate::protocol::transcript::ThreadTranscriptSnapshot;
use crate::protocol::turn::InputItem;
use crate::protocol::turn::TurnInteractionResolveResult;
use crate::protocol::turn::TurnInterruptResult;
use crate::protocol::turn::TurnStartResult;
use crate::protocol::turn::TurnSteerResult;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryAddParams;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryContributionsDto;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryDto;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryListParams;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryMutationDto;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryMutationResult;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryPermissionsSetParams;
use crate::protocol::workspace::WorkspaceAdditionalDirectoryRemoveParams;
use crate::protocol::workspace::WorkspaceFolderDto;
use crate::protocol::workspace::WorkspaceFolderSetEntry;
use crate::protocol::workspace::WorkspaceFoldersSetParams;
use crate::protocol::workspace::WorkspaceFoldersSetResult;
use crate::protocol::workspace::WorkspaceSessionDirectorySelector;
use crate::protocol::workspace::WorkspaceSwitchParams;
use crate::protocol::workspace::WorkspaceSwitchResult;
use crate::protocol::workspace::WorkspaceSwitchTrust;
use crate::protocol::workspace::WorkspaceTrustEntryDto;
use crate::protocol::workspace::WorkspaceTrustForgetParams;
use crate::protocol::workspace::WorkspaceTrustListResult;
use crate::protocol::workspace::WorkspaceTrustReadParams;
use crate::protocol::workspace::WorkspaceTrustReadResult;
use crate::protocol::workspace::WorkspaceTrustSetParams;
use crate::protocol::workspace::WorkspaceTrustSettingDto;
use crate::protocol::workspace::WorkspaceTrustStateDto;
use schemars::JsonSchema;
use ts_rs::Config;
use ts_rs::TS;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::ActionApprovalResponse;
use zeta_protocol::AgentContextContent;
use zeta_protocol::AgentContextMode;
use zeta_protocol::AgentContextSeed;
use zeta_protocol::AgentContextSource;
use zeta_protocol::AgentDefinitionSelectionReason;
use zeta_protocol::AgentInteractionKind;
use zeta_protocol::AgentJoin;
use zeta_protocol::AgentJoinId;
use zeta_protocol::AgentJoinPolicy;
use zeta_protocol::AgentJoinStatus;
use zeta_protocol::AgentMaterializedContext;
use zeta_protocol::AgentMessage;
use zeta_protocol::AgentMessageContent;
use zeta_protocol::AgentMessageId;
use zeta_protocol::AgentMessageProvenance;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::AgentResponse;
use zeta_protocol::AgentRoleSnapshot;
use zeta_protocol::AgentTreeExecutionStatus;
use zeta_protocol::AgentTreeNodeProjection;
use zeta_protocol::AgentTreeProjection;
use zeta_protocol::AgentTreeWaitingReason;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CapabilitySupport;
use zeta_protocol::ContentDigest;
use zeta_protocol::ContentPart;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::ContextCheckpointVerification;
use zeta_protocol::ContextSeedDigest;
use zeta_protocol::ContextSourceDigest;
use zeta_protocol::ContextSourceRange;
use zeta_protocol::DelegatedCapabilityScope;
use zeta_protocol::DelegatedPolicyCeiling;
use zeta_protocol::DelegatedTask;
use zeta_protocol::DelegationArtifactRef;
use zeta_protocol::DelegationId;
use zeta_protocol::DelegationResult;
use zeta_protocol::DelegationResultDigest;
use zeta_protocol::DelegationResultStatus;
use zeta_protocol::DynamicToolCall;
use zeta_protocol::DynamicToolOutput;
use zeta_protocol::DynamicToolResponse;
use zeta_protocol::ForkedAgentContext;
use zeta_protocol::FrozenAgentDefinitionRef;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::ImageDetail;
use zeta_protocol::ImageMediaType;
use zeta_protocol::InteractionCancelReason;
use zeta_protocol::InteractionDeadline;
use zeta_protocol::ItemDelta;
use zeta_protocol::ModelAccess;
use zeta_protocol::ModelCapabilities;
use zeta_protocol::ModelContextUsage;
use zeta_protocol::ModelContextUsageSource;
use zeta_protocol::ModelInputEstimate;
use zeta_protocol::ModelOutputTransport;
use zeta_protocol::ModelUsage;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::ModelUsageTotal;
use zeta_protocol::PendingInteraction;
use zeta_protocol::Personality;
use zeta_protocol::PlanStep;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;
use zeta_protocol::ProcessExecutionOutput;
use zeta_protocol::ProcessExitStatus;
use zeta_protocol::ReasoningEffort;
use zeta_protocol::RequestUserInput;
use zeta_protocol::RequestUserInputResponse;
use zeta_protocol::ReviewTarget;
use zeta_protocol::SandboxDenialOutput;
use zeta_protocol::Session;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::SessionUpdate;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_protocol::SkillVersionSelector;
use zeta_protocol::StableTurnError;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::StreamCursor;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadGoal;
use zeta_protocol::ThreadGoalStatus;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadOrigin;
use zeta_protocol::ThreadSequenceRange;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallCaller;
use zeta_protocol::ToolExecutionAuthority;
use zeta_protocol::ToolMode;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::ToolProfileSnapshot;
use zeta_protocol::ToolReplaySafety;
use zeta_protocol::ToolSourceProvenance;
use zeta_protocol::Turn;
use zeta_protocol::TurnExecutionBinding;
use zeta_protocol::TurnInstructions;
use zeta_protocol::TurnInteraction;
use zeta_protocol::TurnKind;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInputAnswer;
use zeta_protocol::UserInputOption;
use zeta_protocol::UserInputQuestion;
use zeta_protocol::WorkspaceBinding;
use zeta_protocol::WorkspaceTrustId;

/// Selects whether equal scheduling keys exclude or share execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerializationAccess {
    /// Runs alone for its scheduling key.
    Exclusive,
    /// May run with adjacent readers for its scheduling key.
    SharedRead,
}

/// Runtime serialization scope resolved from one typed client-method definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientRequestSerializationScope {
    /// Coordinates App Server-wide state.
    Global { access: SerializationAccess },
    /// Coordinates one durable Session aggregate across connections.
    Session {
        session_id: String,
        access: SerializationAccess,
    },
    /// Coordinates one resource namespace owned by the accepting connection.
    ConnectionResource {
        namespace: &'static str,
        resource_id: String,
        access: SerializationAccess,
    },
}

/// Static serialization declaration stored beside a client method's protocol types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerializationScopeDefinition {
    None,
    GlobalExclusive,
    GlobalSharedRead,
    SessionExclusive,
    SessionSharedRead,
    ResourceExclusive(&'static str),
    ConnectionExclusive(&'static str),
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

    /// Resolves this method's scheduling key from its wire parameters.
    ///
    /// Implementations enqueue equal keys together. Exclusive requests run FIFO, while adjacent
    /// shared reads may run concurrently. Connection resources are additionally namespaced by the
    /// accepting connection in the App Server runtime.
    pub fn serialization_scope(
        &self,
        params: &serde_json::Value,
    ) -> Result<Option<ClientRequestSerializationScope>, SerializationScopeResolutionError> {
        let scope = match self.serialization {
            SerializationScopeDefinition::None => None,
            SerializationScopeDefinition::GlobalExclusive => {
                Some(ClientRequestSerializationScope::Global {
                    access: SerializationAccess::Exclusive,
                })
            }
            SerializationScopeDefinition::GlobalSharedRead => {
                Some(ClientRequestSerializationScope::Global {
                    access: SerializationAccess::SharedRead,
                })
            }
            SerializationScopeDefinition::SessionExclusive => {
                Some(ClientRequestSerializationScope::Session {
                    session_id: serialization_parameter(params, "sessionId")?,
                    access: SerializationAccess::Exclusive,
                })
            }
            SerializationScopeDefinition::SessionSharedRead => {
                Some(ClientRequestSerializationScope::Session {
                    session_id: serialization_parameter(params, "sessionId")?,
                    access: SerializationAccess::SharedRead,
                })
            }
            SerializationScopeDefinition::ResourceExclusive(parameter) => {
                Some(ClientRequestSerializationScope::ConnectionResource {
                    namespace: parameter,
                    resource_id: serialization_parameter(params, parameter)?,
                    access: SerializationAccess::Exclusive,
                })
            }
            SerializationScopeDefinition::ConnectionExclusive(namespace) => {
                Some(ClientRequestSerializationScope::ConnectionResource {
                    namespace,
                    resource_id: String::new(),
                    access: SerializationAccess::Exclusive,
                })
            }
        };
        Ok(scope)
    }
}

/// Returned when request parameters omit the key declared by their method metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerializationScopeResolutionError;

impl std::fmt::Display for SerializationScopeResolutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("request parameters do not contain the declared serialization key")
    }
}

impl std::error::Error for SerializationScopeResolutionError {}

fn serialization_parameter(
    params: &serde_json::Value,
    parameter: &'static str,
) -> Result<String, SerializationScopeResolutionError> {
    let value = params
        .as_object()
        .and_then(|params| params.get(parameter))
        .ok_or(SerializationScopeResolutionError)?;
    if let Some(value) = value.as_str() {
        return Ok(value.to_string());
    }
    serde_json::to_string(value).map_err(|_| SerializationScopeResolutionError)
}

#[derive(Clone, Copy)]
pub struct HostMethodDefinition {
    pub kind: HostMethod,
    pub method: &'static str,
    params_type: fn() -> String,
    result_type: fn() -> String,
}

impl HostMethodDefinition {
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
    dependencies: fn() -> Vec<String>,
    identifier: fn() -> String,
}

impl TypeScriptBinding {
    pub(crate) fn declaration(&self) -> String {
        (self.declaration)()
    }

    pub(crate) fn dependencies(&self) -> Vec<String> {
        (self.dependencies)()
    }

    pub(crate) fn identifier(&self) -> String {
        (self.identifier)()
    }
}

fn type_name<T: TS>() -> String {
    T::name(&Config::default())
}

fn declaration<T: TS>() -> String {
    T::decl(&Config::default())
}

fn dependencies<T: TS + 'static>() -> Vec<String> {
    T::dependencies(&Config::default())
        .into_iter()
        .map(|dependency| dependency.ts_name)
        .collect()
}

fn identifier<T: TS>() -> String {
    T::ident(&Config::default())
}

macro_rules! client_methods {
    (
        $(
            $variant:ident => $method:literal {
                params: $params:ty,
                response: $response:ty,
                serialization: $serialization:ident $(($serialization_key:literal))?,
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
                    serialization: SerializationScopeDefinition::$serialization$(($serialization_key))?,
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
    WorkspaceFoldersSet => "workspace/folders/set" {
        params: WorkspaceFoldersSetParams,
        response: WorkspaceFoldersSetResult,
        serialization: GlobalExclusive,
    },
    WorkspaceAdditionalDirectoryList => "workspace/additionalDirectories/list" {
        params: WorkspaceAdditionalDirectoryListParams,
        response: WorkspaceAdditionalDirectoryListResult,
        serialization: SessionSharedRead,
    },
    WorkspaceAdditionalDirectoryAdd => "workspace/additionalDirectories/add" {
        params: WorkspaceAdditionalDirectoryAddParams,
        response: WorkspaceAdditionalDirectoryMutationResult,
        serialization: SessionExclusive,
    },
    WorkspaceAdditionalDirectoryRemove => "workspace/additionalDirectories/remove" {
        params: WorkspaceAdditionalDirectoryRemoveParams,
        response: WorkspaceAdditionalDirectoryMutationResult,
        serialization: SessionExclusive,
    },
    WorkspaceAdditionalDirectoryPermissionsSet => "workspace/additionalDirectories/permissions/set" {
        params: WorkspaceAdditionalDirectoryPermissionsSetParams,
        response: WorkspaceAdditionalDirectoryMutationResult,
        serialization: SessionExclusive,
    },
    WorkspaceTrustRead => "workspace/trust/read" {
        params: WorkspaceTrustReadParams,
        response: WorkspaceTrustReadResult,
        serialization: GlobalSharedRead,
    },
    WorkspaceTrustList => "workspace/trust/list" {
        params: EmptyParams,
        response: WorkspaceTrustListResult,
        serialization: GlobalSharedRead,
    },
    WorkspaceTrustSet => "workspace/trust/set" {
        params: WorkspaceTrustSetParams,
        response: ConfigCommandResult,
        serialization: GlobalExclusive,
    },
    WorkspaceTrustForget => "workspace/trust/forget" {
        params: WorkspaceTrustForgetParams,
        response: ConfigCommandResult,
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
    ThreadGoalGet => "thread/goal/get" {
        params: ThreadGoalGetParams,
        response: ThreadGoalGetResponse,
        serialization: GlobalSharedRead,
    },
    ThreadGoalSet => "thread/goal/set" {
        params: ThreadGoalSetParams,
        response: ThreadGoalSetResponse,
        serialization: GlobalExclusive,
    },
    ThreadGoalClear => "thread/goal/clear" {
        params: ThreadGoalClearParams,
        response: ThreadGoalClearResponse,
        serialization: GlobalExclusive,
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
    McpServerStatus => "mcp/server/status" {
        params: EmptyParams,
        response: McpServerStatusResult,
        serialization: GlobalSharedRead,
    },
    McpServerConnect => "mcp/server/connect" {
        params: McpServerRuntimeIntentParams,
        response: McpServerRuntimeIntentResult,
        serialization: GlobalExclusive,
    },
    McpServerDisconnect => "mcp/server/disconnect" {
        params: McpServerRuntimeIntentParams,
        response: McpServerRuntimeIntentResult,
        serialization: GlobalExclusive,
    },
    McpOAuthStart => "mcp/oauth/start" {
        params: McpOAuthStartParams,
        response: McpOAuthStartResult,
        serialization: GlobalExclusive,
    },
    McpOAuthComplete => "mcp/oauth/complete" {
        params: McpOAuthCompleteParams,
        response: McpOAuthMutationResult,
        serialization: GlobalExclusive,
    },
    McpOAuthRefresh => "mcp/oauth/refresh" {
        params: McpOAuthMutationParams,
        response: McpOAuthMutationResult,
        serialization: GlobalExclusive,
    },
    McpOAuthRevoke => "mcp/oauth/revoke" {
        params: McpOAuthMutationParams,
        response: McpOAuthMutationResult,
        serialization: GlobalExclusive,
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
    MarketplaceSearch => "marketplace/search" {
        params: MarketplaceSearchParams,
        response: MarketplaceSearchResult,
        serialization: GlobalSharedRead,
    },
    MarketplaceGet => "marketplace/get" {
        params: MarketplaceGetParams,
        response: MarketplacePackageDetailsDto,
        serialization: GlobalSharedRead,
    },
    MarketplaceDownload => "marketplace/download" {
        params: MarketplaceDownloadParams,
        response: MarketplaceArtifactHandleDto,
        serialization: GlobalExclusive,
    },
    MarketplaceInstall => "marketplace/install" {
        params: MarketplaceInstallParams,
        response: MarketplaceInstalledPackageDto,
        serialization: GlobalExclusive,
    },
    MarketplaceUpdate => "marketplace/update" {
        params: MarketplaceUpdateParams,
        response: MarketplaceInstalledPackageDto,
        serialization: GlobalExclusive,
    },
    MarketplaceUninstall => "marketplace/uninstall" {
        params: MarketplaceUninstallParams,
        response: (),
        serialization: GlobalExclusive,
    },
    MarketplaceListInstalled => "marketplace/listInstalled" {
        params: EmptyParams,
        response: MarketplaceListInstalledResult,
        serialization: GlobalSharedRead,
    },
    MarketplaceAcquireCapability => "marketplace/acquireCapability" {
        params: MarketplaceAcquireCapabilityParams,
        response: MarketplaceAcquiredCapabilityDto,
        serialization: GlobalExclusive,
    },
    MarketplaceReleaseCapability => "marketplace/releaseCapability" {
        params: MarketplaceReleaseCapabilityParams,
        response: (),
        serialization: GlobalExclusive,
    },
    MarketplaceOpenResource => "marketplace/openResource" {
        params: MarketplaceOpenResourceParams,
        response: MarketplaceResourceContentDto,
        serialization: GlobalSharedRead,
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
    ProviderList => "provider/list" {
        params: EmptyParams,
        response: ProviderListResult,
        serialization: GlobalSharedRead,
    },
    ProviderApiKeySet => "provider/apiKey/set" {
        params: ProviderApiKeySetParams,
        response: ProviderApiKeySetResult,
        serialization: GlobalExclusive,
    },
    AccountRead => "account/read" {
        params: EmptyParams,
        response: AccountReadResult,
        serialization: GlobalSharedRead,
    },
    AccountLoginStart => "account/login/start" {
        params: AccountLoginStartParams,
        response: AccountLoginStartResult,
        serialization: GlobalExclusive,
    },
    AccountLoginCancel => "account/login/cancel" {
        params: AccountLoginCancelParams,
        response: AccountLoginCancelResult,
        serialization: GlobalExclusive,
    },
    AccountLogout => "account/logout" {
        params: AccountLogoutParams,
        response: AccountLogoutResult,
        serialization: GlobalExclusive,
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
        serialization: ResourceExclusive("skillId"),
    },
    ExtensionList => "extensions/list" {
        params: ExtensionListParams,
        response: ExtensionListResult,
        serialization: GlobalSharedRead,
    },
    ExtensionResourceOpen => "extensions/resource/open" {
        params: ExtensionResourceOpenParams,
        response: ExtensionResourceOpenResult,
        serialization: ResourceExclusive("extensionId"),
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
        serialization: ResourceExclusive("resourceId"),
    },
    ResourceRead => "resource/read" {
        params: ResourceReadParams,
        response: ResourceReadResult,
        serialization: ResourceExclusive("resourceId"),
    },
    ResourceRelease => "resource/release" {
        params: ResourceReleaseParams,
        response: (),
        serialization: ResourceExclusive("resourceId"),
    },
    AttachmentUploadStart => "attachment/upload/start" {
        params: AttachmentUploadStartParams,
        response: AttachmentUploadStartResult,
        serialization: ConnectionExclusive("attachmentIngress"),
    },
    AttachmentUploadWrite => "attachment/upload/write" {
        params: AttachmentUploadWriteParams,
        response: AttachmentUploadWriteResult,
        serialization: ResourceExclusive("uploadId"),
    },
    AttachmentUploadFinish => "attachment/upload/finish" {
        params: AttachmentUploadFinishParams,
        response: AttachmentMaterializeResult,
        serialization: ResourceExclusive("uploadId"),
    },
    AttachmentUploadCancel => "attachment/upload/cancel" {
        params: AttachmentUploadCancelParams,
        response: (),
        serialization: ResourceExclusive("uploadId"),
    },
    AttachmentImportRemote => "attachment/importRemote" {
        params: AttachmentImportRemoteParams,
        response: AttachmentMaterializeResult,
        serialization: ConnectionExclusive("attachmentIngress"),
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
    SyntaxSelectionRanges => "syntax/selectionRanges" {
        params: SyntaxSelectionRangesParams,
        response: SyntaxSelectionRangesResult,
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
    GitRepositories => "git/repositories" {
        params: EmptyParams,
        response: GitRepositoriesResult,
        serialization: GlobalSharedRead,
    },
    GitStatus => "git/status" {
        params: GitRepositoryParams,
        response: GitStatusResult,
        serialization: GlobalSharedRead,
    },
    GitTextDiff => "git/textDiff" {
        params: GitRepositoryParams,
        response: GitTextDiffResult,
        serialization: GlobalSharedRead,
    },
    GitBranchList => "git/branch/list" {
        params: GitRepositoryParams,
        response: GitBranchListResult,
        serialization: GlobalSharedRead,
    },
    GitHistory => "git/history" {
        params: GitRepositoryParams,
        response: GitHistoryResult,
        serialization: GlobalSharedRead,
    },
    GitGraph => "git/graph" {
        params: GitGraphParams,
        response: GitGraphResult,
        serialization: GlobalSharedRead,
    },
    GitCommitChanges => "git/commitChanges" {
        params: GitCommitChangesParams,
        response: GitCommitChangesResult,
        serialization: GlobalSharedRead,
    },
    GitCommitFile => "git/commitFile" {
        params: GitCommitFileParams,
        response: GitCommitFileResult,
        serialization: GlobalSharedRead,
    },
    GitChangeFile => "git/changeFile" {
        params: GitChangeFileParams,
        response: GitChangeFileResult,
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
        params: GitRepositoryParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitPull => "git/pull" {
        params: GitRepositoryParams,
        response: GitOperationResult,
        serialization: GlobalExclusive,
    },
    GitPush => "git/push" {
        params: GitRepositoryParams,
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
    SymbolIndexStatus => "workspace/symbolIndex/status" {
        params: EmptyParams,
        response: SymbolIndexStatusResult,
        serialization: GlobalSharedRead,
    },
    SymbolIndexSearch => "workspace/symbolIndex/search" {
        params: SymbolIndexSearchParams,
        response: SymbolIndexSearchResult,
        serialization: GlobalSharedRead,
    },
    WorkspaceDocumentOverlaySynchronize => "workspace/codeIntelligence/document/synchronize" {
        params: WorkspaceDocumentOverlaySynchronizeParams,
        response: WorkspaceDocumentOverlayStatusResult,
        serialization: GlobalSharedRead,
    },
    WorkspaceDocumentOverlayClose => "workspace/codeIntelligence/document/close" {
        params: WorkspaceDocumentOverlayCloseParams,
        response: WorkspaceDocumentOverlayStatusResult,
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
    FastRegexIndexStatus => "workspace/agentGrep/fastRegex/status" {
        params: EmptyParams,
        response: FastRegexIndexStatusResult,
        serialization: GlobalSharedRead,
    },
    FastRegexIndexRebuild => "workspace/agentGrep/fastRegex/rebuild" {
        params: EmptyParams,
        response: FastRegexIndexStatusResult,
        serialization: GlobalExclusive,
    },
    FastRegexDisableAndDelete => "workspace/agentGrep/fastRegex/disableAndDelete" {
        params: FastRegexDisableAndDeleteParams,
        response: FastRegexDisableAndDeleteResult,
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
    TerminalCreateInSessionDirectory => "terminal/createInSessionDirectory" {
        params: TerminalCreateInSessionDirectoryParams,
        response: TerminalCreateResult,
        serialization: SessionExclusive,
    },
    TerminalAttach => "terminal/attach" {
        params: TerminalAttachParams,
        response: TerminalAttachResult,
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

/// Returns the canonical protocol metadata for an exact client method name.
pub fn client_method_definition(method: &str) -> Option<&'static ClientMethodDefinition> {
    CLIENT_METHODS
        .iter()
        .find(|definition| definition.method == method)
}

macro_rules! host_methods {
    (
        $(
            $variant:ident => $method:literal {
                params: $params:ty,
                response: $response:ty,
            }
        ),+ $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum HostMethod {
            $($variant,)+
        }

        impl HostMethod {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $method,)+
                }
            }
        }

        pub fn host_method(method: &str) -> Option<HostMethod> {
            match method {
                $($method => Some(HostMethod::$variant),)+
                _ => None,
            }
        }

        pub const HOST_METHODS: &[HostMethodDefinition] = &[
            $(
                HostMethodDefinition {
                    kind: HostMethod::$variant,
                    method: $method,
                    params_type: type_name::<$params>,
                    result_type: type_name::<$response>,
                },
            )+
        ];

        #[allow(clippy::enum_variant_names, dead_code)]
        #[derive(JsonSchema)]
        #[serde(tag = "method", content = "params")]
        pub(crate) enum HostRequestSchema {
            $(
                #[serde(rename = $method)]
                $variant($params),
            )+
        }

        #[allow(clippy::enum_variant_names, dead_code)]
        #[derive(JsonSchema)]
        #[serde(tag = "method", content = "result")]
        pub(crate) enum HostResultSchema {
            $(
                #[serde(rename = $method)]
                $variant(Box<$response>),
            )+
        }
    };
}

host_methods! {
    BrowserCreate => "browser/create" {
        params: BrowserCreateParams,
        response: BrowserCreateResult,
    },
    BrowserObserve => "browser/observe" {
        params: BrowserObserveParams,
        response: BrowserObserveResult,
    },
    BrowserPerform => "browser/perform" {
        params: BrowserPerformParams,
        response: BrowserPerformResult,
    },
    BrowserClose => "browser/close" {
        params: BrowserCloseParams,
        response: (),
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
    AccountLoginCompleted => "account/login/completed" {
        params: AccountLoginCompleted,
    },
    AccountUpdated => "account/updated" {
        params: AccountUpdated,
    },
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
    SessionThreadTranscriptUpdate => "session/thread/transcript/update" {
        params: ThreadTranscriptUpdateEnvelope,
    },
    ThreadGoalUpdated => "thread/goal/updated" {
        params: ThreadGoalUpdatedNotification,
    },
    ThreadGoalCleared => "thread/goal/cleared" {
        params: ThreadGoalClearedNotification,
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
    MarketplaceChanged => "marketplace/changed" {
        params: MarketplaceChanged,
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
    LanguageServerState => "language/serverState" {
        params: LanguageServerStateNotification,
    },
}

macro_rules! typescript_bindings {
    ($($type:ty),+ $(,)?) => {
        pub(crate) const TYPESCRIPT_BINDINGS: &[TypeScriptBinding] = &[
            $(
                TypeScriptBinding {
                    declaration: declaration::<$type>,
                    dependencies: dependencies::<$type>,
                    identifier: identifier::<$type>,
                },
            )+
        ];
    };
}

typescript_bindings! {
    AccountDto,
    AccountLoginCancelParams,
    AccountLoginCancelResult,
    AccountLoginCancelStatusDto,
    AccountLoginCompleted,
    AccountLoginCompletionStatusDto,
    AccountLoginFailureDto,
    AccountLoginMethodDto,
    AccountLoginStartParams,
    AccountLoginStartResult,
    AccountLogoutResult,
    AccountLogoutParams,
    AccountLogoutStatusDto,
    AccountReadResult,
    AccountStatusDto,
    AccountUpdated,
    ThreadId,
    SessionId,
    CommandId,
    RequestId,
    StreamInstanceId,
    ItemId,
    ToolCallId,
    ToolName,
    WorkspaceTrustId,
    WorkspaceBinding,
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
    McpSecretDto,
    McpOAuthStartParams,
    McpOAuthStartResult,
    McpOAuthCompleteParams,
    McpOAuthMutationParams,
    McpOAuthMutationResult,
    McpServerRuntimeIntentDto,
    McpServerRuntimeIntentParams,
    McpServerRuntimeIntentResult,
    McpServerRuntimeStateDto,
    McpServerStatusDto,
    McpServerStatusResult,
    MarketplacePackageRefDto,
    MarketplaceArtifactHandleDto,
    MarketplaceCapabilityRefDto,
    MarketplaceResourceRefDto,
    MarketplaceCapabilityKindDto,
    MarketplaceCapabilityDescriptorDto,
    MarketplaceAvailableCapabilityDto,
    MarketplacePackageSummaryDto,
    MarketplaceSearchParams,
    MarketplaceSearchResult,
    MarketplaceGetParams,
    MarketplacePackageDetailsDto,
    MarketplacePackageSourceDto,
    MarketplaceUpstreamRegistryDto,
    MarketplaceUpstreamReferenceDto,
    MarketplaceDownloadParams,
    MarketplaceInstallParams,
    MarketplaceUpdateParams,
    MarketplaceInstallationStateDto,
    MarketplaceInstalledPackageDto,
    MarketplaceListInstalledResult,
    MarketplaceChanged,
    MarketplaceUninstallModeDto,
    MarketplaceUninstallParams,
    MarketplaceAcquireCapabilityParams,
    MarketplaceCapabilityLeaseDto,
    MarketplaceActivationSpecDto,
    MarketplaceSkillActivationSpecDto,
    MarketplaceThemeActivationSpecDto,
    MarketplaceMcpActivationSpecDto,
    MarketplaceMcpTransportDto,
    MarketplaceConnectorActivationSpecDto,
    MarketplaceLanguageActivationSpecDto,
    MarketplaceLocalizationActivationSpecDto,
    MarketplaceExecutableRuntimeDto,
    MarketplaceExecutableActivationSpecDto,
    MarketplaceAcquiredCapabilityDto,
    MarketplaceReleaseCapabilityParams,
    MarketplaceOpenResourceParams,
    MarketplaceResourceContentDto,
    PluginPackageDto,
    PluginListResult,
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
    WorkspaceTrustHostCapability,
    BrowserBinaryPayload,
    BrowserCloseParams,
    BrowserCreateParams,
    BrowserCreateResult,
    BrowserElementTargetDto,
    BrowserObserveParams,
    BrowserObserveResult,
    BrowserPerformActionDto,
    BrowserPerformParams,
    BrowserPerformResult,
    BrowserTextInputTargetDto,
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
    AgentGrepBackendDto,
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
    AgentDefinitionSelectionReason,
    FrozenAgentDefinitionRef,
    AgentRoleSnapshot,
    AgentTreeExecutionStatus,
    AgentTreeWaitingReason,
    AgentTreeNodeProjection,
    AgentTreeProjection,
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
    ExtensionHostOutputEventDto,
    ExtensionHostOutputOperationDto,
    ExtensionHostOutputChannelKindDto,
    ExtensionHostOutputSeverityDto,
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
    ProtocolVersion,
    CapabilityContract,
    ServerCapabilities,
    InitializeParams,
    InitializeResult,
    WorkspaceSwitchParams,
    WorkspaceSwitchResult,
    WorkspaceSwitchTrust,
    WorkspaceFolderDto,
    WorkspaceFolderSetEntry,
    WorkspaceFoldersSetParams,
    WorkspaceFoldersSetResult,
    WorkspaceSessionDirectorySelector,
    WorkspaceAdditionalDirectoryDto,
    WorkspaceAdditionalDirectoryContributionsDto,
    WorkspaceAdditionalDirectoryListParams,
    WorkspaceAdditionalDirectoryListResult,
    WorkspaceAdditionalDirectoryAddParams,
    WorkspaceAdditionalDirectoryRemoveParams,
    WorkspaceAdditionalDirectoryMutationDto,
    WorkspaceAdditionalDirectoryMutationResult,
    WorkspaceAdditionalDirectoryPermissionDto,
    WorkspaceAdditionalDirectoryPermissionsSetParams,
    WorkspaceTrustReadParams,
    WorkspaceTrustReadResult,
    WorkspaceTrustEntryDto,
    WorkspaceTrustListResult,
    WorkspaceTrustSetParams,
    WorkspaceTrustForgetParams,
    WorkspaceTrustSettingDto,
    WorkspaceTrustStateDto,
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
    SessionRewriteResult,
    ThreadGoalStatus,
    ThreadGoal,
    ThreadGoalSetParams,
    ThreadGoalSetResponse,
    ThreadGoalGetParams,
    ThreadGoalGetResponse,
    ThreadGoalClearParams,
    ThreadGoalClearResponse,
    ThreadGoalUpdatedNotification,
    ThreadGoalClearedNotification,
    ThreadSnapshotHistory,
    ThreadHistoryBoundary,
    CapabilitySupport,
    ModelAccess,
    ModelOutputTransport,
    ModelCapabilities,
    ReasoningEffort,
    Personality,
    ModelCatalogEntry,
    ModelListResult,
    ProviderApiKeyDto,
    ProviderApiKeyPolicyDto,
    ProviderApiKeySetParams,
    ProviderApiKeySetResult,
    ProviderCatalogEntryDto,
    ProviderListResult,
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
    ModelContextUsageSource,
    ModelContextUsage,
    ModelInputEstimate,
    ModelUsage,
    ModelUsageTotal,
    ModelUsageSummary,
    ToolMode,
    ToolProfileSnapshot,
    ReviewTarget,
    TurnKind,
    TurnInstructions,
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
    TurnExecutionBinding,
    ThreadEvent,
    PlanStepStatus,
    PlanStep,
    PlanUpdate,
    StreamCursor,
    ItemDelta,
    ThreadUpdate,
    ThreadUpdateEnvelope,
    ThreadTranscriptEntry,
    ThreadTranscriptSnapshot,
    ThreadTranscriptChange,
    ThreadTranscriptUpdateEnvelope,
    InputItem,
    TurnStartResult,
    TurnSteerResult,
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
    SyntaxSelectionRangeDto,
    SyntaxSelectionRangesParams,
    SyntaxSelectionRangesResult,
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
    LanguageServerMessageSourceDto,
    LanguageServerMessageNotification,
    LanguageServerProgressNotification,
    LanguageServerStateDto,
    LanguageServerStateNotification,
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
    GitRepositoryParams,
    GitRepositoryDto,
    GitRepositoriesResult,
    GitRepositoryChangeDto,
    GitStatusResult,
    GitStatusChanged,
    GitBranchDto,
    GitBranchListResult,
    GitCommitSummaryDto,
    GitHistoryResult,
    GitRemoteProviderDto,
    GitRepositoryIdentityDto,
    GitRemoteDto,
    GitReferenceKindDto,
    GitReferenceDto,
    GitGraphParams,
    GitGraphResult,
    GitCommitChangesParams,
    GitCommitChangeDto,
    GitCommitChangesResult,
    GitCommitFileParams,
    GitCommitFileContentDto,
    GitCommitFileResult,
    GitChangeFileComparisonDto,
    GitChangeFileParams,
    GitChangeFileResult,
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
    FastRegexIndexStatusResult,
    FastRegexDisableAndDeleteParams,
    FastRegexDisableAndDeleteResult,
    LocalIndexClearOutcomeDto,
    SemanticCodeIndexStateDto,
    SemanticCodeIndexStatusDto,
    CodeIndexSearchParams,
    CodeIndexChunkSpanDto,
    CodeIndexSearchHitDto,
    CodeIndexSearchResult,
    SymbolIndexStateDto,
    SymbolIndexStatusResult,
    SymbolIndexSearchParams,
    SymbolKindDto,
    SymbolIndexSearchHitDto,
    SymbolIndexSearchResult,
    WorkspaceDocumentOverlaySynchronizeParams,
    WorkspaceDocumentOverlayCloseParams,
    WorkspaceDocumentOverlayStatusResult,
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
    TerminalLifecycle,
    TerminalCreateParams,
    TerminalCreateInSessionDirectoryParams,
    TerminalCreateResult,
    TerminalReconnectLease,
    TerminalAttachParams,
    TerminalAttachResult,
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

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
