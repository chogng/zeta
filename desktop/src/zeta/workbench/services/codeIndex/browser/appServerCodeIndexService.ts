import type { CodeIndexStatusResult, ConfigCommandResult, ConfigReadResult, ProviderConfigDto, SemanticCodeIndexAutomaticContextDto, SemanticCodeIndexSelectionDto } from "../../../../../../generated/app-server/types.js";
import type { ICodeIndexApi } from "../../../../platform/codeIndex/common/codeIndexApi.js";
import type { CodeIndexConfigurationCommandResult, CodeIndexConfigurationSnapshot, CodeIndexProviderConfiguration, CodeIndexStatus, ICodeIndexService, SemanticCodeIndexAutomaticContext, SemanticCodeIndexSelection } from "../../../../platform/codeIndex/common/codeIndexService.js";

export class AppServerCodeIndexService implements ICodeIndexService {
  constructor(private readonly api: ICodeIndexApi) {}

  async readConfig(): Promise<CodeIndexConfigurationSnapshot> { return projectConfiguration(await this.api.readConfig()); }

  async configureProvider(config: CodeIndexProviderConfiguration, expectedRevision: number): Promise<CodeIndexConfigurationCommandResult> {
    return projectCommandResult(await this.api.configureProvider({ commandId: commandId("provider"), expectedRevision, config: providerConfigurationDto(config) }));
  }

  async configure(selection: SemanticCodeIndexSelection, automaticContext: SemanticCodeIndexAutomaticContext, expectedRevision: number): Promise<CodeIndexConfigurationCommandResult> {
    return projectCommandResult(await this.api.configure({ commandId: commandId("configure"), expectedRevision, selection: selectionDto(selection), automaticContext: automaticContext as SemanticCodeIndexAutomaticContextDto }));
  }

  async authorize(expectedRevision: number): Promise<CodeIndexConfigurationCommandResult> {
    return projectCommandResult(await this.api.authorize({ commandId: commandId("authorize"), expectedRevision }));
  }

  async revoke(expectedRevision: number): Promise<CodeIndexConfigurationCommandResult> {
    return projectCommandResult(await this.api.revoke({ commandId: commandId("revoke"), expectedRevision }));
  }

  async status(): Promise<CodeIndexStatus> { return projectStatus(await this.api.status()); }

  async cancel(): Promise<CodeIndexStatus> { return projectStatus(await this.api.cancel()); }

  async retry(): Promise<CodeIndexStatus> { return projectStatus(await this.api.retry()); }
}

function projectConfiguration(config: ConfigReadResult): CodeIndexConfigurationSnapshot {
  return {
    revision: config.revision,
    generation: config.generation,
    providers: Object.fromEntries(Object.entries(config.providers).map(([id, provider]) => [id, projectProviderConfiguration(provider)])),
    semanticCodeIndex: {
      selection: projectSelection(config.semanticCodeIndex.selection),
      automaticContext: config.semanticCodeIndex.automaticContext,
      activeWorkspaceAuthorized: config.semanticCodeIndex.activeWorkspaceAuthorized,
    },
  };
}

function projectProviderConfiguration(config: ProviderConfigDto): CodeIndexProviderConfiguration {
  return { provider: config.provider, baseUrl: config.baseUrl, maxOutputTokens: config.maxOutputTokens, modelContext: config.modelContext ? Object.fromEntries(Object.entries(config.modelContext).map(([model, context]) => [model, { ...context }])) : undefined };
}

function providerConfigurationDto(config: CodeIndexProviderConfiguration): ProviderConfigDto {
  return { provider: config.provider, baseUrl: config.baseUrl, maxOutputTokens: config.maxOutputTokens, modelContext: config.modelContext ? Object.fromEntries(Object.entries(config.modelContext).map(([model, context]) => [model, { ...context }])) : undefined };
}

function projectSelection(selection: SemanticCodeIndexSelectionDto): SemanticCodeIndexSelection {
  return selection.type === "disabled" ? { type: "disabled" } : { type: "remote", models: { embeddingModel: { ...selection.models.embeddingModel }, rerankModel: selection.models.rerankModel ? { ...selection.models.rerankModel } : selection.models.rerankModel } };
}

function selectionDto(selection: SemanticCodeIndexSelection): SemanticCodeIndexSelectionDto {
  return selection.type === "disabled" ? { type: "disabled" } : { type: "remote", models: { embeddingModel: { ...selection.models.embeddingModel }, rerankModel: selection.models.rerankModel ? { ...selection.models.rerankModel } : selection.models.rerankModel } };
}

function projectCommandResult(result: ConfigCommandResult): CodeIndexConfigurationCommandResult {
  return { revision: result.revision, generation: result.generation, disposition: result.disposition };
}

function projectStatus(status: CodeIndexStatusResult): CodeIndexStatus {
  return { ...status, semantic: { ...status.semantic } };
}

function commandId(operation: string): string {
  return `desktop-code-index-${operation}-${crypto.randomUUID()}`;
}
